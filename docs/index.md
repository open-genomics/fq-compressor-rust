---
layout: home
---

<script setup>
import { onMounted } from 'vue'
import { useRouter } from 'vitepress'

const router = useRouter()

onMounted(() => {
  const lang = typeof navigator !== 'undefined'
    ? (navigator.language || navigator.userLanguage || 'en').toLowerCase()
    : 'en'
  
  // Redirect to Chinese for zh-* locales, English for others
  const target = lang.startsWith('zh') ? '/zh/' : '/en/'
  router.go(target)
})
</script>

<div class="loading">
  <p>Detecting language... / 正在检测语言...</p>
</div>

<style>
.loading {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: 50vh;
  color: var(--vp-c-text-2);
}
</style>
