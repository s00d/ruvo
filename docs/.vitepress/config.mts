import { defineConfig } from 'vitepress'
import { pluginsNav, pluginsSidebar } from './plugins-nav.generated'

// Local: `/`. GitHub Pages: set SOVA_DOCS_BASE=/sova/ in CI.
const docsBase = process.env.SOVA_DOCS_BASE || '/'
const ogImage = 'https://s00d.github.io/sova/og.png'

export default defineConfig({
  title: 'Sova',
  description: 'Express-like HTTP framework for Rust',
  base: docsBase,
  ignoreDeadLinks: [/^sova_/],
  lastUpdated: true,
  head: [
    ['link', { rel: 'icon', href: `${docsBase}favicon.svg?v=7`, type: 'image/svg+xml' }],
    ['link', { rel: 'icon', href: `${docsBase}favicon-32.png?v=7`, type: 'image/png', sizes: '32x32' }],
    ['link', { rel: 'apple-touch-icon', href: `${docsBase}apple-touch-icon.png?v=7` }],
    ['meta', { name: 'theme-color', content: '#0f766e' }],
    ['meta', { property: 'og:type', content: 'website' }],
    ['meta', { property: 'og:title', content: 'Sova' }],
    [
      'meta',
      {
        property: 'og:description',
        content: 'Express-like HTTP framework for Rust',
      },
    ],
    ['meta', { property: 'og:image', content: ogImage }],
    ['meta', { name: 'twitter:card', content: 'summary_large_image' }],
    ['meta', { name: 'twitter:image', content: ogImage }],
  ],
  themeConfig: {
    logo: { src: '/logo.png?v=7', alt: 'Sova' },
    siteTitle: 'Sova',
    nav: [
      { text: 'Guide', link: '/guide/getting-started' },
      { text: 'Plugins', items: [...pluginsNav] },
      { text: 'Plugin SDK', link: '/api/plugin-sdk' },
      { text: 'Examples', link: '/examples' },
      { text: 'Performance', link: '/guide/performance' },
      { text: 'GitHub', link: 'https://github.com/s00d/sova' },
    ],
    sidebar: {
      '/guide/': [
        {
          text: 'Guide',
          items: [
            { text: 'Getting started', link: '/guide/getting-started' },
            { text: 'Concepts', link: '/guide/concepts' },
            { text: 'Configuration', link: '/guide/configuration' },
            { text: 'cargo sovax', link: '/guide/cargo-sovax' },
            { text: 'Performance', link: '/guide/performance' },
          ],
        },
      ],
      '/plugins/': [...pluginsSidebar],
      '/api/': [
        {
          text: 'API',
          items: [{ text: 'Plugin SDK', link: '/api/plugin-sdk' }],
        },
      ],
    },
    socialLinks: [{ icon: 'github', link: 'https://github.com/s00d/sova' }],
    search: { provider: 'local' },
    editLink: {
      pattern: 'https://github.com/s00d/sova/edit/master/docs/:path',
    },
  },
})
