# 真实语料压缩/吞吐证据

收尾关账测量：用两份公开 FASTQ 前缀切片（与 C++ 侧 `fq-compressor/docs/real-corpus.md` **同一切片、同一 sha256**）
验证 Rust 端 round-trip 无损、压缩比与吞吐，并给出 C++ 横向参考。切片不入库，用 `scripts/fetch_real_corpus.sh` 重建。

环境：WSL2（Linux 6.18 / 12 核），Rust release 构建，`--threads 8`，默认压缩级别。共享机器、测量期间有并发任务，
墙钟波动 ±20–30%（与 C++ 侧一样只用来下结论，不作 A/B 基准）。吞吐按未压缩 FASTQ 字节 / 墙钟。

## 切片

| 切片 | 来源 | 仪器 / 策略 | 记录 | 碱基 | 文件 | SHA-256 |
|---|---|---|---:|---:|---:|---|
| `SRR2962693_1.head200k.fastq` | [SRR2962693](https://www.ebi.ac.uk/ena/browser/view/SRR2962693) R1 前 200k | Illumina WXS，126 bp 定长 | 200 000 | 25.20 Mbp | 53.96 MiB | `6e38214ac7ebe0c2ba981a616672f695606d456d9dd1066f80f546109039ad9c` |
| `DRR171398_1.head4k.fastq` | [DRR171398](https://www.ebi.ac.uk/ena/browser/view/DRR171398) 前 4k | 人类 MinION | 4 000 | 65.36 Mbp | 124.77 MiB | `748c3d34cdd31996e7ddb7472cd7d941d6744fc4df5d131df33e9ce07e643f1e` |

重建：

```bash
./scripts/fetch_real_corpus.sh        # 输出到 corpus/，被 .gitignore 忽略
```

## 无损验证（round-trip）

| 切片 | 模式 | 压缩输出（磁盘字节） | 解压后 `cmp` |
|---|---:|---|
| Illumina | archive（默认 reorder） | 14,036,446 | 与输入逐字节一致（需 `--original-order` 恢复原序） |
| Illumina | archive（`--reorder false`） | 12,508,029 | 与输入逐字节一致（直接 cmp） |
| Illumina | pipeline | 14,036,446 | 与输入逐字节一致（`--original-order`） |
| Illumina | streaming | 12,507,647 | 与输入逐字节一致（直接 cmp） |
| ONT | archive | 66,014,018 | 与输入逐字节一致 |
| ONT | pipeline | 66,014,018 | 与输入逐字节一致 |
| ONT | streaming | 66,014,018 | 与输入逐字节一致 |

`verify`：archive 全量 verify 通过（如 Illumina 2 blocks / 200 000 reads；ONT 1 block / 4 000 reads）。
压缩确定性：两次相同输入逐字节相同，仅头部时间戳 1 字节不同（已实测 `cmp`）。

## 压缩比与吞吐

磁盘压缩比 = 输入字节 / 归档文件字节。压缩/解压吞吐 = 输入字节 / 墙钟（并发负载下测得，波动大）。

| 切片 | 模式 | 磁盘比 | 压缩 s | 压缩 MiB/s | 解压 s | 解压 MiB/s |
|---|---:|---:|---:|---:|---:|---:|
| Illumina WXS | archive（reorder on） | 4.03× | 11.05 | 4.9 | 2.08 | 26.0 |
| Illumina WXS | archive（reorder off） | 4.52× | 1.93 | 28.0 | 2.14 | 25.3 |
| Illumina WXS | pipeline | 4.03× | 11.19 | 4.8 | 2.12 | 25.5 |
| Illumina WXS | streaming | 4.52× | 3.75 | 14.4 | 2.65 | 20.4 |
| 人类 MinION | archive / pipeline / streaming | 1.98× | 9.3–9.6 | 13.0 | 9.7–10.2 | 12.2–12.9 |

> Illumina archive 解压 2.08s 为 archive 顺序；`--original-order` 恢复原序约 3.5s（需重排 map 换序写出）。

### 与 C++ 侧横向参考

C++ 侧（`fq-compressor/docs/real-corpus.md`，Ryzen 7 5800H / Clang 18 release，L1 zstd）同一切片的数字：

| 切片 | C++ 压缩比 | C++ 压缩 MiB/s | C++ 解压 MiB/s |
|---|---:|---:|---:|
| Illumina WXS | 4.15× | 87.0 | 150 |
| 人类 MinION | 1.96× | 33.5 | 65 |

注意：**两侧不是同机、不同压缩级别、不同线程模型**，绝对数不可直接比。可比的是**结论**：

1. **压缩比一致量级**：Rust 4.03–4.52× vs C++ 4.15×（Illumina）、Rust 1.98× vs C++ 1.96×（MinION）——同语料下
   压缩比结论吻合，Rust 无格式级信息损失（round-trip 逐字节一致）。
2. **短读重排差异**：Rust 默认 reorder 得到 4.03×（比关闭更差），C++ 的 4.15× 是空间有序直压。详见
   `docs/hotspot-report.md` 发现 1——Rust 的重排对真实 WXS 是净亏，下一刀应做自适应/默认关闭。
3. **长读都是 ~1.98×**：两侧一致，质量流近满字母表是压缩比天花板（与 C++ 结论相同）。

## 结论

- 两份真实切片（Illumina WXS 短读 + 人类 MinION 长读）× 三种模式，compress → decompress → `cmp` 全部逐字节一致。
- 压缩比与 C++ 侧同语料结论吻合（短读 ~4×，长读 ~1.98×）；Rust 吞吐为 debug 级可接受，热点集中在重排与单 block 串行（见 hotspot-report）。
- 生产可用性评估：无损链路成立；`--reorder false` / `--streaming` 在真实短读上更快且更小，建议作为默认起点。
