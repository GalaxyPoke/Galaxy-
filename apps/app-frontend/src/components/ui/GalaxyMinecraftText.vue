<script setup lang="ts">
/**
 * Renders a string with Minecraft '&x' / '§x' codes as coloured spans. Text is
 * only ever interpolated (no v-html). On the light theme the very light
 * Minecraft colours (white, yellow, aqua, gray…) are mapped to darker tones that
 * stay readable on white cards; dark themes use the original palette.
 */
import { computed } from 'vue'

import { parseMinecraftText } from '@/galaxy/minecraft-format'

const props = defineProps<{
    text: string
}>()

const segments = computed(() => parseMinecraftText(props.text))
</script>

<template>
    <span class="mc-text"
        ><span
            v-for="(segment, index) in segments"
            :key="index"
            :class="[
                segment.color ? `mc-c-${segment.color}` : null,
                {
                    'mc-bold': segment.bold,
                    'mc-italic': segment.italic,
                    'mc-underline': segment.underline,
                    'mc-strike': segment.strikethrough,
                    'mc-underline-strike': segment.underline && segment.strikethrough,
                },
            ]"
            >{{ segment.text }}</span
        ></span
    >
</template>

<style scoped>
/* Light theme (default): darker tones of the Minecraft palette, readable on white. */
.mc-c-0 {
    color: #1f2328;
}
.mc-c-1 {
    color: #1e3a8a;
}
.mc-c-2 {
    color: #166534;
}
.mc-c-3 {
    color: #0f6e78;
}
.mc-c-4 {
    color: #9f1239;
}
.mc-c-5 {
    color: #86198f;
}
.mc-c-6 {
    color: #b45309;
}
.mc-c-7 {
    color: #5b6370;
}
.mc-c-8 {
    color: #464c55;
}
.mc-c-9 {
    color: #3651c9;
}
.mc-c-a {
    color: #15803d;
}
.mc-c-b {
    color: #0e7490;
}
.mc-c-c {
    color: #dc2626;
}
.mc-c-d {
    color: #c026d3;
}
.mc-c-e {
    color: #a16207;
}
/* White is the "normal" text colour in Minecraft chat. */
.mc-c-f {
    color: inherit;
}

/* Dark themes: the original Minecraft colours. */
:global(.dark-mode .mc-text .mc-c-0),
:global(.oled-mode .mc-text .mc-c-0) {
    color: #9ca3af;
}
:global(.dark-mode .mc-text .mc-c-1),
:global(.oled-mode .mc-text .mc-c-1) {
    color: #6b7cff;
}
:global(.dark-mode .mc-text .mc-c-2),
:global(.oled-mode .mc-text .mc-c-2) {
    color: #00aa00;
}
:global(.dark-mode .mc-text .mc-c-3),
:global(.oled-mode .mc-text .mc-c-3) {
    color: #00aaaa;
}
:global(.dark-mode .mc-text .mc-c-4),
:global(.oled-mode .mc-text .mc-c-4) {
    color: #e04848;
}
:global(.dark-mode .mc-text .mc-c-5),
:global(.oled-mode .mc-text .mc-c-5) {
    color: #c052c0;
}
:global(.dark-mode .mc-text .mc-c-6),
:global(.oled-mode .mc-text .mc-c-6) {
    color: #ffaa00;
}
:global(.dark-mode .mc-text .mc-c-7),
:global(.oled-mode .mc-text .mc-c-7) {
    color: #aaaaaa;
}
:global(.dark-mode .mc-text .mc-c-8),
:global(.oled-mode .mc-text .mc-c-8) {
    color: #777777;
}
:global(.dark-mode .mc-text .mc-c-9),
:global(.oled-mode .mc-text .mc-c-9) {
    color: #5555ff;
}
:global(.dark-mode .mc-text .mc-c-a),
:global(.oled-mode .mc-text .mc-c-a) {
    color: #55ff55;
}
:global(.dark-mode .mc-text .mc-c-b),
:global(.oled-mode .mc-text .mc-c-b) {
    color: #55ffff;
}
:global(.dark-mode .mc-text .mc-c-c),
:global(.oled-mode .mc-text .mc-c-c) {
    color: #ff5555;
}
:global(.dark-mode .mc-text .mc-c-d),
:global(.oled-mode .mc-text .mc-c-d) {
    color: #ff55ff;
}
:global(.dark-mode .mc-text .mc-c-e),
:global(.oled-mode .mc-text .mc-c-e) {
    color: #ffff55;
}

.mc-bold {
    font-weight: 700;
}
.mc-italic {
    font-style: italic;
}
.mc-underline {
    text-decoration: underline;
}
.mc-strike {
    text-decoration: line-through;
}
.mc-underline-strike {
    text-decoration: underline line-through;
}
</style>
