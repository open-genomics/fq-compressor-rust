<script setup lang="ts">
import { useData } from 'vitepress'
const { site } = useData()
</script>

<template>
  <div class="not-found">
    <div class="not-found-content">
      <h1 class="code">404</h1>
      <h2 class="title">Page Not Found</h2>
      <p class="desc">
        The page you're looking for doesn't exist in the fqc documentation.
      </p>
      <div class="actions">
        <a href="/en/" class="action primary">English Docs</a>
        <a href="/zh/" class="action">中文文档</a>
        <a href="https://github.com/LessUp/fq-compressor-rust" class="action">GitHub</a>
      </div>
    </div>
  </div>
</template>

<style scoped>
.not-found {
  display: flex;
  justify-content: center;
  align-items: center;
  min-height: calc(100vh - 64px);
  padding: 2rem;
}

.not-found-content {
  text-align: center;
  max-width: 480px;
}

.code {
  font-size: 8rem;
  font-weight: 700;
  line-height: 1;
  margin: 0;
  color: var(--vp-c-brand-1);
  opacity: 0.5;
}

.title {
  font-size: 2rem;
  font-weight: 600;
  margin: 1rem 0;
  color: var(--vp-c-text-1);
}

.desc {
  font-size: 1rem;
  color: var(--vp-c-text-2);
  margin-bottom: 2rem;
}

.actions {
  display: flex;
  flex-wrap: wrap;
  justify-content: center;
  gap: 0.75rem;
}

.action {
  display: inline-block;
  padding: 0.5rem 1rem;
  font-size: 0.875rem;
  font-weight: 500;
  border-radius: 6px;
  text-decoration: none;
  border: 1px solid var(--vp-c-border);
  color: var(--vp-c-text-1);
  transition: all 0.15s ease;
}

.action:hover {
  border-color: var(--vp-c-brand-1);
  color: var(--vp-c-brand-1);
}

.action.primary {
  background: var(--vp-c-brand-1);
  border-color: var(--vp-c-brand-1);
  color: white;
}

.action.primary:hover {
  background: var(--vp-c-brand-2);
}

@media (max-width: 640px) {
  .code {
    font-size: 5rem;
  }

  .title {
    font-size: 1.5rem;
  }
}
</style>
