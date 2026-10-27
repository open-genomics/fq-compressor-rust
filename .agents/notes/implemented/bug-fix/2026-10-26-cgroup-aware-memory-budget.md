# Agent Note: 自动内存预算感知 cgroup 限额

Status: implemented

## Problem

`get_available_memory_linux()` 只读取 `/proc/meminfo` 的 `MemAvailable`。在容器
中该值是宿主机可用内存，不是本 cgroup 的可用额度：实测宿主机约 1.9 TiB，而
当前 cgroup 的 `memory.max` 只有 200 GiB。按 75% 自动预算时，程序可能把额度
估到远高于 cgroup 上限，在大文件 archive/pipeline 摄入或 decode/reorder 路径
上触发 OOM kill，而不是在预算检查时提前失败。

## Decision

Linux 自动内存探测返回以下两者的较小值：

- `/proc/meminfo` 的 `MemAvailable`；
- cgroup 限额减去当前用量后的剩余额度。

实现优先读取 cgroup v2 的 `/sys/fs/cgroup/memory.max` 与
`/sys/fs/cgroup/memory.current`；文件不存在时回退到 cgroup v1 的
`memory.limit_in_bytes` 与 `memory.usage_in_bytes`。`memory.max` 为 `max`
或数值不可读时保留宿主机探测；cgroup 用量不可读时按 0 计。剩余额度最低为
1 MiB，由既有 `MIN_*_MEMORY_MB` 继续做最终下限裁剪。

显式 `--memory-limit` 仍完全覆盖自动预算，不读取 cgroup。macOS 路径不变；
无 cgroup 的 Linux 主机仍按 `/proc/meminfo` 工作。

## Alternatives considered

- **只使用 cgroup limit，不扣当前用量** — 实现更简单，但当同一容器已有其他
  进程占用内存时仍会高估；否决。
- **只使用宿主机 `MemAvailable`（现状）** — 对无容器主机正确，但在当前
  cgroup 环境中会把 200 GiB 误判成约 1.4 TiB 的 75% 预算；否决。
- **要求用户通过环境变量提供额度** — 不自动，容易遗漏且会产生新的配置面；
  否决。
- **解析 `/proc/self/cgroup` 构造嵌套 cgroup 路径** — 覆盖更完整，但当前
  部署与常见容器都把 cgroup 根挂在 `/sys/fs/cgroup`，引入路径构造和嵌套
  命名空间处理不符合本项目的低维护定位；暂不采用，若出现非根挂载再补。

## Consequences

- **收益**：容器内 `--memory-limit 0` 的预算与真实上限一致或更保守；大文件
  路径更可能在分配前以 `ResourceLimit` 失败，而不是被 OOM kill。
- **代价**：`memory.current` 包含页缓存等 cgroup 记账内存，自动预算可能比
  宿主机可用量更保守；需要接近上限的吞吐时应显式传 `--memory-limit`。
- **边界**：只处理 cgroup 根挂载；无 cgroup 文件或值为 `max` 时保留宿主机
  探测。v1 的超大“无限”限额会与宿主机探测取较小值，不会扩大预算。

## Verification

新增 `linux_memory_tests` 覆盖 cgroup 数值解析、`max` 处理、用量扣减和
饱和边界；本机 cgroup v2 的 200 GiB 限额由实际运行路径读取。`cargo test --lib
--tests`、`cargo fmt`、`cargo clippy`、`cargo doc` 的验证结果记录在本次改动
的交付说明中。
