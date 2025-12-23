# Storage Interop Design

## 目标与范围

**目标**：Rust 预编译合约与 Solidity 合约在 EVM storage 层互操作。  
**范围**：storage layout、packed 规则、slot 计算、读写 API 规范。  
**非目标**：ABI 编解码（明确 storage != ABI）。

## 背景与问题定义

当前需要在 Rust 预编译合约中直接读取/写入 Solidity 合约的 storage。两者必须对齐 storage
layout、slot 计算与 packed 规则，否则会产生读写偏移、数据覆盖或不可恢复的状态污染。本设计文档
定义统一的布局与规则，确保 Rust 侧和 Solidity 侧在 storage 层一致。

## 设计目标与约束

- 与 Solidity 的 storage layout 与 packed 规则严格兼容。
- 支持 Solidity 常见类型组合（基础类型、struct/array/mapping/bytes/string）。
- 读写 API 必须显式依赖 layout/packing 规则，避免“看似正确”的错误读写。
- 与 Tempo/reth 当前存储访问路径兼容。

## 类型映射与布局体系

### Solidity → Rust 类型对照表

| Solidity 类型 | Rust 类型 | storage 规则摘要 |
| --- | --- | --- |
| `uint8` | `u8` | 1 字节，可能 packed |
| `uint16` | `u16` | 2 字节，可能 packed |
| `uint32` | `u32` | 4 字节，可能 packed |
| `uint64` | `u64` | 8 字节，可能 packed |
| `uint128` | `u128` | 16 字节，可能 packed |
| `uint256` / `int256` | `U256` | 32 字节，独占 slot |
| `address` | `Address` | 20 字节，可能 packed |
| `bool` | `bool` | 1 字节，可能 packed |

### bytes/string/array/struct/mapping 的存储规则摘要

- `bytes` / `string`：短数据（<= 31 bytes）内联存储到 slot，长度与数据共存；长数据放入
  `keccak256(slot)` 开始的连续 slots，slot 内存长度。
- 固定长度数组（static array）：元素按顺序连续存放，遵循元素类型的 packed 规则。
- 动态数组（dynamic array）：slot 存长度，数据从 `keccak256(slot)` 起连续存放。
- `struct`：字段按声明顺序存放，遵循 packed 规则与 slot 递增。
- `mapping`：不占用连续 slot；元素位置为 `keccak256(key . slot)`。

## Storage slot 计算规则

1. **静态字段 slot 递增**
   - 从 slot 0 开始，按声明顺序为每个字段分配 slot。
   - 若字段可 packed，与前一字段共享 slot；否则切换到下一 slot。

2. **packed slot 计算（按字节偏移）**
   - 同一 slot 内按声明顺序从低地址字节开始放置字段。
   - 记录字段的 byte offset 与 length，用于读写时的位移与掩码。

3. **动态数组**
   - `slot` 仅存储长度 `len`。
   - 数据起始点：`keccak256(slot)`。
   - 第 `i` 个元素位于 `keccak256(slot) + i * element_slot_size`。

4. **mapping**
   - 对于 `mapping(K => V)`，元素位置为 `keccak256(encode(K) . slot)`。
   - `encode(K)` 遵循 Solidity 的 32-byte 左填充规则（等价于 ABI 对 key 的静态编码）。

## Packed 规则与位操作

- **字节对齐**：packed 字段在同一 slot 内紧凑排列，按 1 字节粒度对齐。
- **偏移方向**：与 Solidity 一致，字段按声明顺序从 slot 的低位字节（低地址）开始填充。
- **覆盖规则**：写入 packed 字段必须保留同一 slot 其他字段的位：
  1. 读取原 slot 值 `old`。
  2. 计算字段掩码 `mask`（基于字节偏移与长度）。
  3. 写入值 `v` 左移到偏移位置，更新 `new = (old & !mask) | (v_shifted & mask)`。
- **右对齐/左对齐兼容性**：数值类型在字段内部按无符号整数存储，写入前进行
  大端对齐到字段字节宽度，再按 byte offset 放置，以确保与 Solidity 编译器输出一致。

## 复杂类型（struct/array/mapping/bytes）布局

- `struct`：字段按声明顺序处理，每个字段遵循 packed 与 slot 递增规则。
- `static array`：元素顺序连续布局，元素类型如基础类型则可 packed，否则独占 slot。
- `dynamic array`：长度存在 `slot`，数据从 `keccak256(slot)` 起，按元素 slot size 递增。
- `bytes/string`：短数据内联存储；长数据存储于 `keccak256(slot)` 起连续 slots。
- `mapping`：元素存储在 `keccak256(key . slot)`，不影响其他字段 slot 递增。
- `nested struct/mapping/array`：递归应用以上规则，slot 起点以父级计算结果为准。

## Rust 预编译读写 API 设计

### 接口定义

- `read<T>(address, slot) -> T`
- `write<T>(address, slot, value)`

### 语义说明

- 接口不直接理解 Solidity 类型，依赖调用方传入的布局/packed 元数据。
- `slot` 为字段计算后的 slot 基址。
- 对 packed 字段，`read/write` 需要额外的 byte offset 与长度来执行掩码与位移。
- 对动态类型（`bytes/string/array/mapping`），`slot` 表示根 slot，内部根据规则计算子 slot。

## 安全与权限模型

- Rust 预编译读取/写入必须受权限控制，仅允许对目标合约允许的 storage 进行操作。
- 提供白名单或 capability 机制，防止任意 slot 读写导致状态破坏。
- 所有写操作建议记录审计日志（包含 address/slot/value/调用方）。

## 测试矩阵与验证策略

| 场景 | Solidity 写 Rust 读 | Rust 写 Solidity 读 |
| --- | --- | --- |
| struct packed | ✅ | ✅ |
| struct unpacked | ✅ | ✅ |
| static array | ✅ | ✅ |
| dynamic array | ✅ | ✅ |
| mapping | ✅ | ✅ |
| nested struct | ✅ | ✅ |
| bytes/string | ✅ | ✅ |

验证策略：
- 对每类类型生成 Solidity 合约与 Rust 预编译用例，进行双向读写一致性测试。
- 使用已知 slot 计算的黄金向量进行比对。
- 测试覆盖 packed 位写入不覆盖相邻字段。

## 与 Tempo/reth 的差异与兼容性说明

- Tempo/reth 当前 storage 访问通常以 slot 为单位，本设计在其基础上引入 packed
  字段的位级读写处理。
- 兼容现有 `revm` 的 storage 读写接口，仅在上层增加 layout/packing 计算与掩码逻辑。
- 若 Tempo 已有自定义 slot 访问工具，需要补充 packed 字段的掩码支持。

## 后续实现计划

- 拟建 `storage-interop` crate 模块清单：
  - `layout`: Solidity 类型布局解析与 slot/offset 计算
  - `packing`: 掩码/位移/对齐工具
  - `codec`: storage 层编码/解码（不包含 ABI）
  - `types`: Rust 类型与 Solidity 类型映射
  - `tests`: 黄金向量与双向读写用例
- 与 `precompiles`/`revm` 的整合点：
  - `precompiles`: 提供 `read/write` API 的入口与权限检查
  - `revm`: 通过现有 storage 读写接口执行实际读写
