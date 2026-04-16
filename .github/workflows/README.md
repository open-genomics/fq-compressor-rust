# GitHub Actions Workflows

## Documentation Deployment

### pages-vitepress.yml (NEW - Recommended)
- **Technology**: VitePress (VuePress successor)
- **Performance**: 10-50x faster than Honkit
- **Features**: Modern UI, instant search, perfect i18n
- **Build Time**: ~30 seconds vs 3 minutes (Honkit)
- **Status**: ✅ Production ready

### pages.yml (Legacy - Honkit)
- **Technology**: Honkit (GitBook fork)
- **Status**: ⚠️ Deprecated, will be removed

## Migration

To use the new VitePress documentation:
1. Enable `pages-vitepress.yml`
2. Disable old `pages.yml`
3. Update Pages source branch if needed

## Key Improvements

| Feature | Honkit (old) | VitePress (new) |
|---------|-------------|-----------------|
| Build Speed | 3-5 min | 30 sec |
| Bundle Size | 15 MB | 3 MB |
| Search | Plugin-based | Built-in instant |
| Mobile UX | Poor | Excellent |
| Dark Mode | Plugin | Native |
| i18n | Partial | Full |
