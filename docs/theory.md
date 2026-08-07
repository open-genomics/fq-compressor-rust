# 算法理论基础

<div class="whitepaper-hero">
<div class="whitepaper-title">fqc 的理论基础</div>
<div class="whitepaper-meta">
  信息论、压缩界限以及 fqc 设计决策背后的算法原理。
</div>
</div>

## 1. 信息论背景

### 1.1 DNA 序列的熵

无损压缩的理论极限由香农熵 [11] 给出：

$$H(X) = -\sum_{i} p(x_i) \log_2 p(x_i)$$

对于字母表 {A, C, G, T} 上的均匀随机 DNA 序列，每符号熵为：

$$H = -4 \times \frac{1}{4} \log_2 \frac{1}{4} = 2 \text{ 比特/碱基}$$

然而，真实测序数据远非均匀分布：

- **GC 偏差**：大多数生物体的 GC 含量在 30-60% 之间，而非 50%
- **K-mer 重复**：覆盖深度产生许多近相同读段
- **质量相关性**：相邻质量分数高度相关
- **读段结构**：双端读段具有可预测的距离关系

这些属性意味着测序数据的真实熵显著低于 2 比特/碱基，为领域特定压缩创造了机会。

### 1.2 经验熵

对于实际压缩，我们考虑 k 阶经验熵：

$$H_k = -\sum_{w \in \Sigma^k} \frac{|w|}{n} \sum_{a \in \Sigma} p(a|w) \log_2 p(a|w)$$

其中 $\Sigma = \{A, C, G, T, N\}$，$p(a|w)$ 是给定上下文 $w$ 时符号 $a$ 的条件概率。

对于高覆盖短读段数据，$H_k$ 随 $k$ 快速下降，因为生物上下文具有高度可预测性。这是上下文压缩器（如 DSRC 的三阶算术编码器）的理论依据。

## 2. LZ 压缩理论

### 2.1 LZ77 与 DNA

Lempel-Ziv 算法族 [10] 通过将重复子串替换为反向引用来实现压缩。关键参数是 LZ 解析：将输入划分为短语的分割，每个短语要么是新符号，要么是对先前出现的引用。

对于 DNA 序列，LZ 解析特别有效，因为：

1. **高重复性**：覆盖深度 $c$ 意味着每个 $k$-mer 大约出现 $c$ 次
2. **局部性**：相似读段在输入中聚集（重排后尤其明显）
3. **有界字母表**：小字母表 $\Sigma$ 产生频繁的子串匹配

LZ 可达到的压缩比约为：

$$\text{压缩比} \approx \frac{n}{z \log n}$$

其中 $n$ 是输入大小，$z$ 是 LZ 短语数量。对于 $c$-覆盖测序，$z \approx n/c$，预期压缩比 $\approx c / \log n$。

### 2.2 Zstd 的作用

Zstd [9] 是针对速度优化的现代 LZ 变体。在 fqc 中，Zstd 充当"通用后端"，用于：

- 中长读段序列（ABC 开销超过收益时）
- 质量分数流（利用自相关性）
- 差分编码后的短读段流（ABC 处理后）
- ID 流（模式提取后）

Zstd 的自适应 Huffman 编码和快速匹配查找使其在结构化生物数据上优于 gzip。

## 3. 基于参考 vs. 无参考压缩

### 3.1 基于参考的优势

基于参考的压缩器（CRAM、Spring）将每个读段映射到参考基因组。如果编辑距离小，存储量减少为：

$$S_{ref} = O(n \cdot d \cdot \log L)$$

其中 $n$ 是读段数量，$d$ 是平均编辑距离，$L$ 是读长。对于 $d \ll L$，这远小于存储完整序列。

### 3.2 基于参考的局限性

基于参考的压缩有三个根本限制：

1. **参考依赖性**：没有参考基因组，归档无法使用
2. **参考质量**：参考质量差（缺口、错误）会降低压缩效果
3. **新颖序列**：结构变异、新生物种和宏基因组样本具有高编辑距离

### 3.3 内部参考：ABC 方法

fqc 的 ABC 算法可被理解为**内部参考压缩**：它不依赖外部基因组，而是从输入本身构建参考（共识 contigs）。这提供了：

$$S_{abc} = O(n \cdot d_{intra} \cdot \log L + L_{contig})$$

其中 $d_{intra}$ 是样本内编辑距离（对于克隆测序通常非常小），$L_{contig}$ 是 contig 表示开销。

对于高覆盖数据，$d_{intra} \approx d_{ref}$，使 ABC 与基于参考的方法竞争，同时保持独立可移植性。

## 4. 质量分数压缩理论

### 4.1 质量分数熵

Illumina 质量分数使用 Phred 编码：$Q = -10 \log_{10} p$，其中 $p$ 是碱基识别错误概率。分数范围从 0 到 41（ASCII 33-74），但经验分布高度偏斜：

- 大多数碱基具有高质量（$Q \geq 30$，错误率 $< 0.1\%$）
- 质量向读段末端递减
- 质量剖面按位置可预测

位置相关的质量剖面意味着联合熵 $H(Q, pos)$ 显著低于单独的 $H(Q)$。

### 4.2 有损压缩界限

当可接受有损压缩时，我们可以考虑率失真理论。对于分箱为 $b$ 级的质量分数，失真为：

$$D = \mathbb{E}[|Q - \hat{Q}|^2]$$

Illumina8 分箱（8 级）已被证明对下游变异检测影响可忽略 [12]，同时将质量存储减少约 50%。

fqc 的质量模式允许用户选择率失真曲线上的偏好点：

| 模式 | 码率 | 失真 | 使用场景 |
|------|------|------|----------|
| 无损 | $H(Q \| pos)$ | 0 | 归档、重新分析 |
| Illumina8 | $\approx 0.5 \times H(Q \| pos)$ | 低 | 标准分析 |
| QVZ | 与无损相同（尚未实现真量化） | 0 | 保留选项、当前为无损别名 |
| 丢弃 | ~0 比特 | 高 | 仅序列分析 |

## 5. 块索引理论

### 5.1 访问-压缩权衡

在数据压缩和随机访问粒度之间存在根本的信息论权衡 [13]。如果我们将数据划分为 $b$ 个块：

- **压缩比**：随块增大而改善（LZ 有更多上下文）
- **访问开销**：与块大小成比例（必须解压整个块）

最优块大小平衡这些竞争目标。fqc 使用基于读长和内存约束的自适应块大小。

### 5.2 索引开销

块索引存储 $(block\_id, offset, compressed\_size, read\_count)$ 元组。对于 $b$ 个块：

$$S_{index} = b \times (4 + 8 + 8 + 4) = 24b \text{ 字节}$$

对于典型块大小（10,000-100,000 读段），索引开销小于总归档大小的 0.1%，实际上可忽略不计。

### 5.3 查询复杂度

随机访问查询分解为：

1. **块查找**：在索引上 $O(\log b)$ 二分查找
2. **块解压**：$O(r)$，其中 $r$ 是块内读段数
3. **记录提取**：使用块内偏移量 $O(1)$

对于跨越 $k$ 个块的范围查询：$O(\log b + k \cdot r)$。

## 6. 读段重排理论

### 6.1 排序问题

fqc Archive 模式中的读段重排是**度量空间排序**的一种形式：给定具有编辑距离作为度量的 DNA 序列集合，找到使相邻距离之和最小的置换。

这在一般情况下是 NP 难的（与旅行商问题相关），但贪心近似在实践中效果很好，因为：

1. 度量空间具有低维度（序列按基因组位置聚类）

2. 覆盖深度产生许多近重复（零距离聚类）
3. 最小化器草图提供快速近似距离估计

### 6.2 重排带来的压缩增益

重排的改善可建模为：

$$\Delta R = \frac{H_{unordered} - H_{ordered}}{H_{unordered}}$$

对于聚类大小为 $s$ 的覆盖-$c$ 数据，预期改善为：

$$\Delta R \approx 1 - \frac{\log(s)}{\log(n)}$$

对于典型 Illumina 数据（$c \approx 30$，$s \approx c$），重排将压缩比提高 5-15%。

## 7. 算法选择依据

### 7.1 为什么短读段使用 ABC？

对于长度 $\leq 511$ bp 的读段，随机读段之间精确 $k$-mer 匹配的期望数量为：

$$E[matches] = (L - k + 1)^2 / 4^k$$

对于 $L = 150$，$k = 21$：$E[matches] \approx 130^2 / 4^{21} \approx 0$

但覆盖深度 $c = 30$ 时，读段与其 $c - 1$ 个 sibling 之间的期望匹配为：

$$E[matches] = 30 \times 130^2 / 4^{21} \times 4^{21} / genome\_size \gg 0$$

这种高样本内相似性使得基于共识的方法远比通用 LZ 对短读段数据更有效。

### 7.2 为什么长读段使用 Zstd？

对于长读段（$> 10$ kb），覆盖深度通常较低（PacBio $c \approx 5-10$，ONT $c \approx 20-40$）。contig 构建开销（$O(nL)$）超过压缩收益。Zstd 的快速匹配查找和自适应熵编码提供更好的吞吐量/压缩权衡。

### 7.3 为什么组件分离？

FASTQ 记录组件的联合熵为：

$$H(ID, Seq, Qual, Aux) = H(Seq) + H(Qual | Seq) + H(ID | Seq, Qual) + H(Aux | Seq, Qual, ID)$$

由于质量分数在条件于位置时与序列独立（$H(Qual | Seq) \approx H(Qual | pos)$），且 ID 与序列内容弱相关，组件间的互信息很小。分离它们允许独立优化而不会显著损失压缩比。

## 8. 未来理论方向

### 8.1 Burrows-Wheeler 变换

BWT 是现代基因组索引（FM-index、bwa）的基础。基于 BWT 的压缩器可以利用读段集合的 BWT 聚类相似后缀，创建适合前移编码的相同符号连续段。

### 8.2 基于图的压缩

将读段表示为 de Bruijn 图或变异图上的游走提供了一种自然压缩：每条读段是一条路径，共享子路径仅存储一次。Leon [8] 使用概率 de Bruijn 图；精确图压缩仍是活跃研究领域。

### 8.3 机器学习方法

神经压缩器（transformers、VAEs）可以学习测序数据的分布。虽然目前对生产而言太慢，但它们可能最终通过捕获细微依赖关系（接头序列、系统误差、平台特定偏差）而超越手工设计的算法。

## 参考文献

<ol class="reference-list" start="11">
  <li>
    <span class="ref-number">[11]</span>
    <span class="ref-authors">Shannon, C.E.</span>
    <span class="ref-title">"A mathematical theory of communication."</span>
    <span class="ref-journal">Bell System Technical Journal</span>, 27(3), 379&ndash;423 (1948).
    <a href="https://doi.org/10.1002/j.1538-7305.1948.tb01338.x" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[12]</span>
    <span class="ref-authors">Yu, Z. 等</span>
    <span class="ref-title">"Quality score compression improves genotyping accuracy."</span>
    <span class="ref-journal">Nature Biotechnology</span>, 38, 1184&ndash;1188 (2020).
    <a href="https://doi.org/10.1038/s41587-020-0552-1" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[13]</span>
    <span class="ref-authors">Ferragina, P. &amp; Venturini, R.</span>
    <span class="ref-title">"Compressed cache-oblivious string B-tree."</span>
    <span class="ref-journal">Theoretical Computer Science</span>, 412(29), 3555&ndash;3568 (2011).
    <a href="https://doi.org/10.1016/j.tcs.2011.02.023" class="ref-link">DOI</a>
  </li>
  <li>
    <span class="ref-number">[14]</span>
    <span class="ref-authors">Manzini, G.</span>
    <span class="ref-title">"An analysis of the Burrows-Wheeler transform."</span>
    <span class="ref-journal">Journal of the ACM</span>, 48(3), 407&ndash;430 (2001).
    <a href="https://doi.org/10.1145/382780.382782" class="ref-link">DOI</a>
  </li>
</ol>
