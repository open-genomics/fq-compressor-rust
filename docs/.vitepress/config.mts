import { defineConfig } from 'vitepress'
import { withMermaid } from 'vitepress-plugin-mermaid'
import llmstxt from 'vitepress-plugin-llms'

// Dynamic base path for GitHub Pages
const rawBase = process.env.VITEPRESS_BASE
const base = rawBase
  ? rawBase.startsWith('/')
    ? rawBase.endsWith('/') ? rawBase : `${rawBase}/`
    : `/${rawBase}/`
  : '/'

export default withMermaid(defineConfig({
  base,
  title: 'fqc',
  description: '基于 Rust 的块索引 FASTQ 压缩工具',
  cleanUrls: true,
  ignoreDeadLinks: false,
  lastUpdated: true,

  head: [
    ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
    ['meta', { name: 'theme-color', content: '#06b6d4' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'fqc' }],
    ['meta', { property: 'og:image', content: '/og-image.svg' }],
    ['meta', { property: 'og:image:width', content: '1200' }],
    ['meta', { property: 'og:image:height', content: '630' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:image', content: '/og-image.svg' }],
    ['script', { type: 'application/ld+json' }, JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'SoftwareSourceCode',
      name: 'fqc',
      description: '块索引 FASTQ 压缩工具',
      programmingLanguage: 'Rust',
      license: 'https://spdx.org/licenses/GPL-3.0-or-later.html',
      codeRepository: 'https://github.com/LessUp/fq-compressor-rust',
      author: { '@type': 'Person', name: 'LessUp' }
    })]
  ],

  sitemap: {
    hostname: 'https://lessup.github.io/fq-compressor-rust/'
  },

  themeConfig: {
    logo: { src: '/logo.svg', width: 24, height: 24 },
    search: { provider: 'local' },
    editLink: {
      pattern: 'https://github.com/LessUp/fq-compressor-rust/edit/master/docs/:path',
      text: '在 GitHub 上编辑此页'
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/LessUp/fq-compressor-rust' }
    ],
    footer: {
      message: 'fqc 技术白皮书与架构展示',
      copyright: 'GPL-3.0 · fqc 贡献者'
    },
    nav: [
      { text: '白皮书', link: '/whitepaper', activeMatch: '/whitepaper' },
      {
        text: '指南',
        activeMatch: '/guide/',
        items: [
          { text: '快速开始', link: '/guide/quick-start' },
          { text: '安装', link: '/guide/installation' },
          { text: '压缩模式', link: '/guide/modes' },
          { text: 'CLI 参考', link: '/guide/cli' }
        ]
      },
      {
        text: '架构',
        activeMatch: '/architecture/',
        items: [
          { text: '概述', link: '/architecture/' },
          { text: '决策记录', link: '/architecture/decisions/' },
          { text: '性能路线图', link: '/architecture/performance-roadmap' }
        ]
      },
      {
        text: '算法',
        activeMatch: '/algorithms/',
        items: [
          { text: '概述', link: '/algorithms/' },
          { text: 'ABC 深度解析', link: '/algorithms/abc-deep-dive' }
        ]
      },
      {
        text: '参考',
        activeMatch: '/(reference|benchmarks|references|release-notes|comparison|theory)',
        items: [
          { text: '二进制格式规范', link: '/reference/format-spec' },
          { text: '基准测试', link: '/benchmarks/performance-report' },
          { text: '竞品对比', link: '/comparison' },
          { text: '理论基础', link: '/theory' },
          { text: '参考文献', link: '/references/' },
          { text: '发布说明', link: '/release-notes' }
        ]
      }
    ],
    sidebar: {
      '/guide/': [
        {
          text: '指南',
          items: [
            { text: '安装', link: '/guide/installation' },
            { text: '快速开始', link: '/guide/quick-start' },
            { text: '压缩模式', link: '/guide/modes' },
            { text: 'CLI 参考', link: '/guide/cli' }
          ]
        }
      ],
      '/architecture/': [
        {
          text: '架构',
          items: [
            { text: '概述', link: '/architecture/' },
            {
              text: '决策记录',
              collapsed: true,
              items: [
                { text: 'ADR-001: 块索引归档格式', link: '/architecture/decisions/001-block-indexed-format' },
                { text: 'ADR-002: 三种执行模式', link: '/architecture/decisions/002-three-execution-modes' },
                { text: 'ADR-003: 组件级编码', link: '/architecture/decisions/003-component-encoding' }
              ]
            },
            { text: '性能路线图', link: '/architecture/performance-roadmap' }
          ]
        }
      ],
      '/algorithms/': [
        {
          text: '算法',
          items: [
            { text: '概述', link: '/algorithms/' },
            { text: 'ABC 算法深度解析', link: '/algorithms/abc-deep-dive' }
          ]
        }
      ],
      '/benchmarks/': [
        { text: '基准测试', items: [{ text: '性能报告', link: '/benchmarks/performance-report' }] }
      ],
      '/reference/': [
        { text: '参考', items: [{ text: '二进制格式规范', link: '/reference/format-spec' }] }
      ],
      '/references/': [
        { text: '参考文献', items: [{ text: '参考文献与相关工作', link: '/references/' }] }
      ]
    }
  },

  vite: {
    plugins: [llmstxt({})]
  },

  mermaid: {
    theme: { light: 'base', dark: 'dark' },
    themeVariables: {
      primaryColor: '#06b6d4',
      primaryTextColor: '#F9FAFB',
      primaryBorderColor: '#06b6d4',
      lineColor: '#6B7280',
      secondaryColor: '#1F2937',
      tertiaryColor: '#111827',
      fontFamily: 'ui-sans-serif, system-ui, -apple-system, sans-serif',
      fontSize: '14px'
    }
  }
}))
