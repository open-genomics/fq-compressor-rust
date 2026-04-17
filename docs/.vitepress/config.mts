import { defineConfig } from 'vitepress'
import { zh } from './config.zh'

// https://vitepress.dev/reference/site-config
export default defineConfig({
  // Site Metadata
  title: 'fqc',
  titleTemplate: ':title - High-Performance FASTQ Compressor',
  description: 'High-performance FASTQ compressor written in Rust with ABC algorithm',
  
  // Multi-language support
  locales: {
    root: {
      label: 'English',
      lang: 'en',
      link: '/',
    },
    zh: {
      label: '简体中文',
      lang: 'zh-Hans',
      link: '/zh/',
      themeConfig: zh.themeConfig
    }
  },

  // Clean URLs (no .html)
  cleanUrls: true,
  
  // Head - SEO & Performance
  head: [
    ['link', { rel: 'icon', href: '/favicon.svg', type: 'image/svg+xml' }],
    ['link', { rel: 'alternate icon', href: '/favicon.ico', sizes: '32x32' }],
    ['meta', { name: 'theme-color', content: '#646cff' }],
    ['meta', { name: 'og:type', content: 'website' }],
    ['meta', { name: 'og:locale', content: 'en' }],
    ['meta', { name: 'og:site_name', content: 'fqc' }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
  ],

  // Markdown Extensions
  markdown: {
    theme: {
      light: 'github-light',
      dark: 'github-dark'
    },
    lineNumbers: true,
    math: true,
  },

  // Sitemap
  sitemap: {
    hostname: 'https://lessup.github.io/fq-compressor-rust/'
  },

  // Last Updated
  lastUpdated: {
    text: 'Last updated',
    formatOptions: {
      dateStyle: 'medium',
      timeStyle: 'short'
    }
  },

  // Theme Configuration
  themeConfig: {
    // Logo
    logo: { src: '/logo.svg', width: 24, height: 24 },

    // Navigation
    nav: [
      { text: 'Guide', link: '/guide/what-is-fqc', activeMatch: '/guide/' },
      { text: 'Architecture', link: '/architecture/', activeMatch: '/architecture/' },
      { text: 'Algorithms', link: '/algorithms/', activeMatch: '/algorithms/' },
      { 
        text: 'Changelog', 
        items: [
          { text: 'v0.1.1', link: '/changelog/v0.1.1' },
          { text: 'v0.1.0', link: '/changelog/v0.1.0' },
          { text: 'All Versions', link: '/changelog/' }
        ]
      }
    ],

    // Sidebar
    sidebar: {
      '/guide/': { base: '/guide/', items: sidebarGuide() },
      '/architecture/': { base: '/architecture/', items: sidebarArchitecture() },
      '/algorithms/': { base: '/algorithms/', items: sidebarAlgorithms() }
    },

    // Footer
    footer: {
      message: 'Released under the GPL-3.0 License.',
      copyright: 'Copyright  2024-present fqc contributors'
    },

    // Social Links
    socialLinks: [
      { icon: 'github', link: 'https://github.com/LessUp/fq-compressor-rust' },
    ],

    // Local Search
    search: {
      provider: 'local',
      options: {
        detailedView: true,
        placeholder: 'Search documentation...',
      }
    },

    // Edit Link
    editLink: {
      pattern: 'https://github.com/LessUp/fq-compressor-rust/edit/master/docs/:path',
      text: 'Edit this page on GitHub'
    },

    // Outline
    outline: {
      label: 'On this page',
      level: [2, 3]
    },
  },
})

// Sidebar: Guide
function sidebarGuide(): any[] {
  return [
    {
      text: 'Getting Started',
      collapsed: false,
      items: [
        { text: 'What is fqc?', link: 'what-is-fqc' },
        { text: 'Installation', link: 'installation' },
        { text: 'Quick Start', link: 'quick-start' }
      ]
    },
    {
      text: 'CLI',
      collapsed: false,
      items: [
        { text: 'Compression', link: 'cli/compress' },
        { text: 'Decompression', link: 'cli/decompress' },
        { text: 'Information', link: 'cli/info' },
        { text: 'Verification', link: 'cli/verify' }
      ]
    },
    {
      text: 'Features',
      collapsed: false,
      items: [
        { text: 'Streaming Mode', link: 'features/streaming' },
        { text: 'Pipeline Mode', link: 'features/pipeline' },
        { text: 'Paired-End Support', link: 'features/paired-end' },
        { text: 'Quality Modes', link: 'features/quality-modes' }
      ]
    },
    {
      text: 'Performance',
      collapsed: false,
      items: [
        { text: 'Benchmarks', link: 'performance/benchmarks' },
        { text: 'Tuning Guide', link: 'performance/tuning' }
      ]
    },
    {
      text: 'Development',
      collapsed: false,
      items: [
        { text: 'Contributing', link: 'development/contributing' },
        { text: 'Building from Source', link: 'development/building' }
      ]
    }
  ]
}

// Sidebar: Architecture
function sidebarArchitecture(): any[] {
  return [
    {
      text: 'Architecture',
      items: [
        { text: 'Overview', link: '' },
        { text: 'Data Flow', link: 'data-flow' },
        { text: 'Module Structure', link: 'modules' },
        { text: 'Block Format', link: 'block-format' }
      ]
    },
    {
      text: 'Core Components',
      collapsed: false,
      items: [
        { text: 'FASTQ Parser', link: 'components/parser' },
        { text: 'Block Compressor', link: 'components/block-compressor' },
        { text: 'Global Analyzer', link: 'components/global-analyzer' },
        { text: 'Quality Compressor', link: 'components/quality-compressor' },
        { text: 'I/O Layer', link: 'components/io' },
        { text: 'Pipeline', link: 'components/pipeline' }
      ]
    },
    {
      text: 'File Format',
      collapsed: false,
      items: [
        { text: 'Format Specification', link: 'format-spec' },
        { text: 'Magic Header', link: 'magic-header' },
        { text: 'Block Header', link: 'block-header' },
        { text: 'Reorder Map', link: 'reorder-map' },
        { text: 'Footer & Index', link: 'footer-index' }
      ]
    }
  ]
}

// Sidebar: Algorithms
function sidebarAlgorithms(): any[] {
  return [
    {
      text: 'Algorithms',
      items: [
        { text: 'Overview', link: '' },
        { text: 'Strategy Selection', link: 'strategy-selection' }
      ]
    },
    {
      text: 'Sequence Compression',
      collapsed: false,
      items: [
        { text: 'ABC Algorithm', link: 'abc' },
        { text: 'Consensus Building', link: 'consensus' },
        { text: 'Delta Encoding', link: 'delta-encoding' },
        { text: 'Zstd Codec', link: 'zstd' }
      ]
    },
    {
      text: 'Quality Compression',
      collapsed: false,
      items: [
        { text: 'SCM Overview', link: 'scm' },
        { text: 'Context Models', link: 'context-models' },
        { text: 'Arithmetic Coding', link: 'arithmetic-coding' }
      ]
    },
    {
      text: 'Reordering',
      collapsed: false,
      items: [
        { text: 'Minimizer Algorithm', link: 'minimizer' },
        { text: 'Reorder Map Encoding', link: 'reorder-map-encoding' }
      ]
    },
    {
      text: 'Optimization',
      collapsed: false,
      items: [
        { text: 'Paired-End Optimization', link: 'pe-optimization' },
        { text: 'ID Compression', link: 'id-compression' }
      ]
    }
  ]
}
