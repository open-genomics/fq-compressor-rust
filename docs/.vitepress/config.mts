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
  description: 'A block-indexed FASTQ compression tool in Rust',
  cleanUrls: true,
  ignoreDeadLinks: false,

  head: [
    ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
    ['meta', { name: 'theme-color', content: '#4f46e5' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:site_name', content: 'fqc' }]
  ],

  sitemap: {
    hostname: 'https://lessup.github.io/fq-compressor-rust/'
  },

  // i18n configuration
  locales: {
    zh: {
      label: '简体中文',
      lang: 'zh-CN',
      link: '/zh/',
      title: 'fqc',
      description: '基于 Rust 的块索引 FASTQ 压缩工具',
      themeConfig: {
        nav: [
          { text: '快速开始', link: '/zh/guide/quick-start', activeMatch: '/zh/guide/' },
          { text: 'CLI 参考', link: '/zh/guide/cli', activeMatch: '/zh/guide/' },
          { text: '架构', link: '/zh/architecture/', activeMatch: '/zh/architecture/' },
          { text: '算法', link: '/zh/algorithms/', activeMatch: '/zh/algorithms/' },
          { text: '基准测试', link: '/zh/benchmarks/performance-report', activeMatch: '/zh/benchmarks/' },
          { text: '发布说明', link: '/zh/release-notes', activeMatch: '/zh/release-notes/' },
        ],
        sidebar: {
          '/zh/guide/': [
            {
              text: '指南',
              items: [
                { text: '安装', link: '/zh/guide/installation' },
                { text: '快速开始', link: '/zh/guide/quick-start' },
                { text: '压缩模式', link: '/zh/guide/modes' },
                { text: 'CLI 参考', link: '/zh/guide/cli' }
              ]
            }
          ],
          '/zh/architecture/': [
            {
              text: '架构',
              items: [
                { text: '概述', link: '/zh/architecture/' },
                { text: '性能路线图', link: '/zh/architecture/performance-roadmap' },
                { text: '决策记录', link: '/zh/architecture/decisions/' }
              ]
            }
          ],
          '/zh/algorithms/': [
            {
              text: '算法',
              items: [
                { text: '概述', link: '/zh/algorithms/' },
                { text: 'ABC 算法深度解析', link: '/zh/algorithms/abc-deep-dive' }
              ]
            }
          ],
          '/zh/benchmarks/': [
            {
              text: '基准测试',
              items: [
                { text: '性能报告', link: '/zh/benchmarks/performance-report' }
              ]
            }
          ],
          '/zh/reference/': [
            {
              text: '参考',
              items: [
                { text: '二进制格式规范', link: '/zh/reference/format-spec' }
              ]
            }
          ]
        }
      }
    },
    en: {
      label: 'English',
      lang: 'en-US',
      link: '/en/',
      title: 'fqc',
      description: 'A block-indexed FASTQ compression tool in Rust',
      themeConfig: {
        nav: [
          { text: 'Quick Start', link: '/en/guide/quick-start', activeMatch: '/en/guide/' },
          { text: 'CLI Reference', link: '/en/guide/cli', activeMatch: '/en/guide/' },
          { text: 'Architecture', link: '/en/architecture/', activeMatch: '/en/architecture/' },
          { text: 'Algorithms', link: '/en/algorithms/', activeMatch: '/en/algorithms/' },
          { text: 'Benchmarks', link: '/en/benchmarks/performance-report', activeMatch: '/en/benchmarks/' },
          { text: 'Release Notes', link: '/en/release-notes', activeMatch: '/en/release-notes/' },
        ],
        sidebar: {
          '/en/guide/': [
            {
              text: 'Guide',
              items: [
                { text: 'Installation', link: '/en/guide/installation' },
                { text: 'Quick Start', link: '/en/guide/quick-start' },
                { text: 'Compression Modes', link: '/en/guide/modes' },
                { text: 'CLI Reference', link: '/en/guide/cli' }
              ]
            }
          ],
          '/en/architecture/': [
            {
              text: 'Architecture',
              items: [
                { text: 'Overview', link: '/en/architecture/' },
                { text: 'Performance Roadmap', link: '/en/architecture/performance-roadmap' },
                { text: 'Architecture Decisions', link: '/en/architecture/decisions/' }
              ]
            }
          ],
          '/en/algorithms/': [
            {
              text: 'Algorithms',
              items: [
                { text: 'Overview', link: '/en/algorithms/' },
                { text: 'ABC Algorithm Deep Dive', link: '/en/algorithms/abc-deep-dive' }
              ]
            }
          ],
          '/en/benchmarks/': [
            {
              text: 'Benchmarks',
              items: [
                { text: 'Performance Report', link: '/en/benchmarks/performance-report' }
              ]
            }
          ],
          '/en/reference/': [
            {
              text: 'Reference',
              items: [
                { text: 'Binary Format Specification', link: '/en/reference/format-spec' }
              ]
            }
          ]
        }
      }
    }
  },

  themeConfig: {
    logo: { src: '/logo.svg', width: 24, height: 24 },
    search: {
      provider: 'local'
    },
    editLink: {
      pattern: 'https://github.com/LessUp/fq-compressor-rust/edit/master/docs/:path',
      text: 'Edit this page on GitHub'
    },
    socialLinks: [
      { icon: 'github', link: 'https://github.com/LessUp/fq-compressor-rust' }
    ],
    footer: {
      message: 'A focused FASTQ compression tool for bioinformatics.',
      copyright: 'GPL-3.0 · fqc contributors'
    }
  },

  vite: {
    plugins: [llmstxt()]
  },

  mermaid: {
    // Mermaid configuration
    theme: 'default'
  }
}))
