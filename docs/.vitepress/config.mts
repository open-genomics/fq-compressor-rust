import { defineConfig } from 'vitepress'

export default defineConfig({
  title: 'fqc',
  titleTemplate: ':title · FASTQ compression in Rust',
  description: 'A focused documentation site for the fqc FASTQ compressor.',
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
  themeConfig: {
    logo: { src: '/logo.svg', width: 24, height: 24 },
    nav: [
      { text: 'Quick Start', link: '/guide/quick-start' },
      { text: 'CLI', link: '/guide/cli' },
      { text: 'Architecture', link: '/architecture/' },
      { text: 'Algorithms', link: '/algorithms/' },
      { text: 'Release Notes', link: '/release-notes' },
      { text: 'GitHub', link: 'https://github.com/LessUp/fq-compressor-rust' }
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Installation', link: '/guide/installation' },
            { text: 'Quick Start', link: '/guide/quick-start' },
            { text: 'CLI Reference', link: '/guide/cli' }
          ]
        }
      ],
      '/architecture/': [
        {
          text: 'Architecture',
          items: [{ text: 'Overview', link: '/architecture/' }]
        }
      ],
      '/algorithms/': [
        {
          text: 'Algorithms',
          items: [{ text: 'Overview', link: '/algorithms/' }]
        }
      ]
    },
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
      message: 'Focused docs for a focused FASTQ compression tool.',
      copyright: 'GPL-3.0 · fqc contributors'
    }
  }
})
