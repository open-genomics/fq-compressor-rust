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
      '/zh/algorithms/': { base: '/zh/algorithms/', items: sidebarAlgorithmsZh() },
      '/zh/changelog/': { base: '/zh/changelog/', items: sidebarChangelogZh() }
    },

    footer: {
      message: '基于 GPL-3.0 许可证发布。',
      copyright: 'Copyright © 2024-present fqc contributors'
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

function sidebarGuideZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '快速开始',
      items: [
        { text: '什么是 FQC?', link: 'what-is-fqc' },
        { text: '安装', link: 'installation' },
        { text: '快速上手', link: 'quick-start' },
      ],
    },
    {
      text: 'CLI 命令',
      items: [
        { text: '压缩', link: 'cli/compress' },
      ],
    },
  ]
}

function sidebarArchitectureZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '架构概览',
      items: [
        { text: '概览', link: '' },
      ],
    },
  ]
}

function sidebarAlgorithmsZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '压缩算法',
      items: [
        { text: '概览', link: '' },
      ],
    },
  ]
}

function sidebarChangelogZh(): DefaultTheme.SidebarItem[] {
  return [
    {
      text: '更新日志',
      items: [
        { text: '概览', link: '' },
        { text: 'v0.1.1', link: 'v0.1.1' },
        { text: 'v0.1.0', link: 'v0.1.0' },
      ],
    },
  ]
}

// Default export for VitePress
export default zh
