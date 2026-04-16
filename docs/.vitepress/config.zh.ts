import { defineConfig, type DefaultTheme } from 'vitepress'

// Chinese config export
export const zh = defineConfig({
  lang: 'zh-Hans',
  description: 'Rust 编写的高性能 FASTQ 压缩器，采用 ABC 算法',

  themeConfig: {
    nav: [
      { text: '指南', link: '/zh/guide/what-is-fqc', activeMatch: '/zh/guide/' },
      { text: '架构', link: '/zh/architecture/', activeMatch: '/zh/architecture/' },
      { text: '算法', link: '/zh/algorithms/', activeMatch: '/zh/algorithms/' },
      {
        text: '更新日志',
        items: [
          { text: 'v0.1.1', link: '/zh/changelog/v0.1.1' },
          { text: 'v0.1.0', link: '/zh/changelog/v0.1.0' },
          { text: '所有版本', link: '/zh/changelog/' }
        ]
      }
    ],

    sidebar: {
      '/zh/guide/': { base: '/zh/guide/', items: sidebarGuideZh() },
      '/zh/architecture/': { base: '/zh/architecture/', items: sidebarArchitectureZh() },
      '/zh/algorithms/': { base: '/zh/algorithms/', items: sidebarAlgorithmsZh() }
    },

    footer: {
      message: '基于 GPL-3.0 许可证发布。',
      copyright: 'Copyright  2024-present fqc contributors'
    },

    lastUpdated: {
      text: '最后更新于',
    },

    outline: {
      label: '本页目录'
    },

    editLink: {
      pattern: 'https://github.com/LessUp/fq-compressor-rust/edit/master/docs/:path',
      text: '在 GitHub 上编辑此页'
    },

    docFooter: {
      prev: '上一页',
      next: '下一页'
    },

    search: {
      provider: 'local',
      options: {
        detailedView: true,
        placeholder: '搜索文档...',
        translations: {
          button: {
            buttonText: '搜索',
            buttonAriaLabel: '搜索文档'
          },
          modal: {
            noResultsText: '没有找到结果',
            resetButtonTitle: '重置搜索',
            footer: {
              selectText: '选择',
              navigateText: '导航',
              closeText: '关闭'
            }
          }
        }
      }
    },
  }
})

// Default export for VitePress
export default zh
function sidebarGuideZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '快速开始',
      collapsed: false,
      items: [
        { text: '什么是 fqc?', link: 'what-is-fqc' },
        { text: '安装', link: 'installation' },
        { text: '快速上手', link: 'quick-start' }
      ]
    },
    {
      text: 'CLI 命令',
      collapsed: false,
      items: [
        { text: '压缩', link: 'cli/compress' },
        { text: '解压', link: 'cli/decompress' },
        { text: '查看信息', link: 'cli/info' },
        { text: '验证', link: 'cli/verify' }
      ]
    },
    {
      text: '功能特性',
      collapsed: false,
      items: [
        { text: '流式模式', link: 'features/streaming' },
        { text: '流水线模式', link: 'features/pipeline' },
        { text: '配对端支持', link: 'features/paired-end' },
        { text: '质量模式', link: 'features/quality-modes' }
      ]
    },
    {
      text: '性能优化',
      collapsed: false,
      items: [
        { text: '基准测试', link: 'performance/benchmarks' },
        { text: '调优指南', link: 'performance/tuning' }
      ]
    },
    {
      text: '开发',
      collapsed: false,
      items: [
        { text: '贡献指南', link: 'development/contributing' },
        { text: '从源码构建', link: 'development/building' }
      ]
    }
  ]
}

function sidebarArchitectureZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '架构设计',
      items: [
        { text: '概述', link: '' },
        { text: '数据流', link: 'data-flow' },
        { text: '模块结构', link: 'modules' },
        { text: '块格式', link: 'block-format' }
      ]
    },
    {
      text: '核心组件',
      collapsed: false,
      items: [
        { text: 'FASTQ 解析器', link: 'components/parser' },
        { text: '块压缩器', link: 'components/block-compressor' },
        { text: '全局分析器', link: 'components/global-analyzer' },
        { text: '质量压缩器', link: 'components/quality-compressor' },
        { text: 'I/O 层', link: 'components/io' },
        { text: '流水线', link: 'components/pipeline' }
      ]
    },
    {
      text: '文件格式',
      collapsed: false,
      items: [
        { text: '格式规范', link: 'format-spec' },
        { text: '魔数头部', link: 'magic-header' },
        { text: '块头部', link: 'block-header' },
        { text: '重排映射', link: 'reorder-map' },
        { text: '尾部与索引', link: 'footer-index' }
      ]
    }
  ]
}

function sidebarAlgorithmsZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '算法',
      items: [
        { text: '概述', link: '' },
        { text: '策略选择', link: 'strategy-selection' }
      ]
    },
    {
      text: '序列压缩',
      collapsed: false,
      items: [
        { text: 'ABC 算法', link: 'abc' },
        { text: '共识构建', link: 'consensus' },
        { text: '增量编码', link: 'delta-encoding' },
        { text: 'Zstd 编解码', link: 'zstd' }
      ]
    },
    {
      text: '质量压缩',
      collapsed: false,
      items: [
        { text: 'SCM 概述', link: 'scm' },
        { text: '上下文模型', link: 'context-models' },
        { text: '算术编码', link: 'arithmetic-coding' }
      ]
    },
    {
      text: '重排序',
      collapsed: false,
      items: [
        { text: 'Minimizer 算法', link: 'minimizer' },
        { text: '重排映射编码', link: 'reorder-map-encoding' }
      ]
    },
    {
      text: '优化',
      collapsed: false,
      items: [
        { text: '配对端优化', link: 'pe-optimization' },
        { text: 'ID 压缩', link: 'id-compression' }
      ]
    }
  ]
}
