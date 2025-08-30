<template>
  <div class="root" :class="`-theme-${theme.theme.value}`" :style="{ '--theme': theme.theme.value }">
    <main class="main">
      <InfiniteList
          class="lines"
          :data="lines"
          width="100%"
          :height="540"
          :itemSize="25"
          :scrollToIndex="scrollToIndex"
          scrollToAlignment="end"
          v-slot="{ item }"
      >
        <code class="line" :title="item">{{ item }}</code>
      </InfiniteList>
    </main>
  </div>
</template>

<script setup lang="ts">
import {invoke, Channel} from '@tauri-apps/api/core'
import {nextTick, onMounted, ref} from "vue";
import {useTheme} from "./composables/useTheme.ts";
import InfiniteList from 'vue3-infinite-list';
import {useEventListener} from "@vueuse/core";

type LogEvent = {
  event: 'log',
  data: {
    line: string
  }
} | {
  event: 'print',
  data: {
    line: string
  }
} | {
  event: 'finished',
  data: {}
};

const theme = useTheme();
const params = new URLSearchParams(location.search);
const target = params.get("target");

onMounted(() => {
  console.log('target', target);
})

// getSizeAndPositionForIndex が例外上げるので空行で埋めとく
const emptyLines = 50
const getEmpty = () => Array.from({length: emptyLines}, () => undefined);
const lines = ref<(string | undefined)[]>(getEmpty())
const scrollToIndex = ref<number | undefined>()

// InfiniteList が内部で watch 使いやがるので lines.value の参照を差し替えつつパフォーマンス悪化を防ぎたい
let i_ = 0
const l_ = [lines.value, [...lines.value]]
const upd = () => lines.value = l_[i_ = (i_ + 1) % 2]
const push = (str: string) => {
  const i = lines.value.slice(0, emptyLines).findIndex(x => typeof x === 'undefined')
  if (i === -1) {
    l_[0].push(str)
    l_[1].push(str)
  } else {
    l_[0][i] = l_[1][i] = str
  }
  upd()
}

const logEvent = new Channel<LogEvent>();
logEvent.onmessage = (ev) => {
  switch (ev.event) {
    case 'finished':
      break;
    default:
      push(ev.data.line)
  }
};

const clear = async () => {
  scrollToIndex.value = 0;
  await nextTick()
  l_.splice(0, 2, getEmpty(), getEmpty())
  upd()
  console.log(`[${target}]`, 'log cleared')
}

useEventListener(window.document, 'keydown', async (e) => {
  if (e.key === 'l' && (e.metaKey || e.ctrlKey)) {
    await clear();
    e.stopPropagation();
    e.preventDefault();
  }
})

;(async () => {
  while (true) {
    try {
      await nextTick()
      await invoke('get_logs', {target, logEvent})
      break
    } catch (_e) {
    }
  }
})()
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