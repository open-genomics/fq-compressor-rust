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

  // Rewrites to fix llms.txt "Untitled" entries
  rewrites: {
    'en/index.md': 'en/index.md',
    'zh/index.md': 'zh/index.md'
  },

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
    // JSON-LD structured data
    ['script', { type: 'application/ld+json' }, JSON.stringify({
      '@context': 'https://schema.org',
      '@type': 'SoftwareSourceCode',
      name: 'fqc',
      description: 'Block-indexed FASTQ compression tool for bioinformatics',
      programmingLanguage: 'Rust',
      license: 'https://spdx.org/licenses/GPL-3.0-or-later.html',
      codeRepository: 'https://github.com/LessUp/fq-compressor-rust',
      author: {
        '@type': 'Person',
        name: 'LessUp'
      }
    })]
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
          {
            text: '白皮书',
            link: '/zh/whitepaper',
            activeMatch: '/zh/whitepaper'
          },
          {
            text: '指南',
            activeMatch: '/zh/guide/',
            items: [
              { text: '快速开始', link: '/zh/guide/quick-start' },
              { text: '安装', link: '/zh/guide/installation' },
              { text: '压缩模式', link: '/zh/guide/modes' },
              { text: 'CLI 参考', link: '/zh/guide/cli' }
            ]
          },
          {
            text: '架构',
            activeMatch: '/zh/architecture/',
            items: [
              { text: '概述', link: '/zh/architecture/' },
              { text: '决策记录', link: '/zh/architecture/decisions/' },
              { text: '性能路线图', link: '/zh/architecture/performance-roadmap' }
            ]
          },
          {
            text: '算法',
            activeMatch: '/zh/algorithms/',
            items: [
              { text: '概述', link: '/zh/algorithms/' },
              { text: 'ABC 深度解析', link: '/zh/algorithms/abc-deep-dive' }
            ]
          },
          {
            text: '参考',
            activeMatch: '/zh/(reference|benchmarks|references|release-notes|comparison|theory)',
            items: [
              { text: '二进制格式规范', link: '/zh/reference/format-spec' },
              { text: '基准测试', link: '/zh/benchmarks/performance-report' },
              { text: '竞品对比', link: '/zh/comparison' },
              { text: '理论基础', link: '/zh/theory' },
              { text: '参考文献', link: '/zh/references/' },
              { text: '发布说明', link: '/zh/release-notes' }
            ]
          }
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
                {
                  text: '决策记录',
                  collapsed: true,
                  items: [
                    { text: 'ADR-001: 块索引归档格式', link: '/zh/architecture/decisions/001-block-indexed-format' },
                    { text: 'ADR-002: 三种执行模式', link: '/zh/architecture/decisions/002-three-execution-modes' },
                    { text: 'ADR-003: 组件级编码', link: '/zh/architecture/decisions/003-component-encoding' }
                  ]
                },
                { text: '性能路线图', link: '/zh/architecture/performance-roadmap' }
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
          ],
          '/zh/references/': [
            {
              text: '参考文献',
              items: [
                { text: '参考文献与相关工作', link: '/zh/references/' }
              ]
            }
          ],
          '/zh/whitepaper': [
            {
              text: '技术白皮书',
              items: [
                { text: '概述', link: '/zh/whitepaper' }
              ]
            }
          ],
          '/zh/comparison': [
            {
              text: '竞品对比',
              items: [
                { text: '对比分析', link: '/zh/comparison' }
              ]
            }
          ],
          '/zh/theory': [
            {
              text: '理论基础',
              items: [
                { text: '算法理论', link: '/zh/theory' }
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
          {
            text: 'Whitepaper',
            link: '/en/whitepaper',
            activeMatch: '/en/whitepaper'
          },
          {
            text: 'Guide',
            activeMatch: '/en/guide/',
            items: [
              { text: 'Quick Start', link: '/en/guide/quick-start' },
              { text: 'Installation', link: '/en/guide/installation' },
              { text: 'Compression Modes', link: '/en/guide/modes' },
              { text: 'CLI Reference', link: '/en/guide/cli' }
            ]
          },
          {
            text: 'Architecture',
            activeMatch: '/en/architecture/',
            items: [
              { text: 'Overview', link: '/en/architecture/' },
              { text: 'Decision Records', link: '/en/architecture/decisions/' },
              { text: 'Performance Roadmap', link: '/en/architecture/performance-roadmap' }
            ]
          },
          {
            text: 'Algorithms',
            activeMatch: '/en/algorithms/',
            items: [
              { text: 'Overview', link: '/en/algorithms/' },
              { text: 'ABC Deep Dive', link: '/en/algorithms/abc-deep-dive' }
            ]
          },
          {
            text: 'Reference',
            activeMatch: '/en/(reference|benchmarks|references|release-notes|comparison|theory)',
            items: [
              { text: 'Binary Format Spec', link: '/en/reference/format-spec' },
              { text: 'Benchmarks', link: '/en/benchmarks/performance-report' },
              { text: 'Comparison', link: '/en/comparison' },
              { text: 'Theory', link: '/en/theory' },
              { text: 'References', link: '/en/references/' },
              { text: 'Release Notes', link: '/en/release-notes' }
            ]
          }
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
                {
                  text: 'Decision Records',
                  collapsed: true,
                  items: [
                    { text: 'ADR-001: Block-Indexed Format', link: '/en/architecture/decisions/001-block-indexed-format' },
                    { text: 'ADR-002: Three Execution Modes', link: '/en/architecture/decisions/002-three-execution-modes' },
                    { text: 'ADR-003: Component Encoding', link: '/en/architecture/decisions/003-component-encoding' }
                  ]
                },
                { text: 'Performance Roadmap', link: '/en/architecture/performance-roadmap' }
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
          ],
          '/en/references/': [
            {
              text: 'References',
              items: [
                { text: 'References & Related Work', link: '/en/references/' }
              ]
            }
          ],
          '/en/whitepaper': [
            {
              text: 'Technical Whitepaper',
              items: [
                { text: 'Overview', link: '/en/whitepaper' }
              ]
            }
          ],
          '/en/comparison': [
            {
              text: 'Comparison',
              items: [
                { text: 'Competitive Analysis', link: '/en/comparison' }
              ]
            }
          ],
          '/en/theory': [
            {
              text: 'Theory',
              items: [
                { text: 'Algorithmic Theory', link: '/en/theory' }
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
      message: 'Technical Whitepaper & Architecture Showcase for fqc.',
      copyright: 'GPL-3.0 · fqc contributors'
    }
  },

  vite: {
    plugins: [llmstxt({
      // Exclude language switch intermediate pages
      ignoreFiles: ['en.md', 'zh.md']
    })]
  },

  mermaid: {
    theme: {
      light: 'base',
      dark: 'dark'
    },
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
