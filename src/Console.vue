<template>
  <div class="root" :class="`-theme-${theme.theme.value}`" :style="{ '--theme': theme.theme.value }">
    <main class="main">
      <InfiniteList
          class="lines"
          :data
          width="100%"
          :height="540"
          :itemSize="25"
          :scrollToIndex="scrollToIndex"
          scrollToAlignment="end"
          v-slot="{ item, index }"
      >
        <code class="line" :title="item">{{ index + 1 }} : {{ item }}</code>
      </InfiniteList>
    </main>
  </div>
</template>

<script setup lang="ts">
import {onMounted, ref} from "vue";
import {useTheme} from "./composables/useTheme.ts";
import InfiniteList from 'vue3-infinite-list';

const theme = useTheme();
const params = new URLSearchParams(location.search);
const target = params.get("target");

onMounted(() => {
  console.log('target', target);
})

const data = Array.from({length: 100_000}, (_, _i) => `12345678901234567890123456789012345678901234567890123456789012345678901234567890`)
const scrollToIndex = ref(data.length - 1)
</script>

<style>
.root {
  color: var(--color-text, #333);
  background: var(--color-background, #fff);

  width: 100%;
  height: 100%;
  overflow: hidden;
}

.root.-theme-light {
  --color-text: #333;
  --color-background: #fff;
}

.root.-theme-dark {
  --color-text: #eee;
  --color-background: #222;
}
</style>

<style scoped>
.line {
  text-wrap: nowrap;
  overflow: hidden;
  font-family: monospace, serif;
}
</style>