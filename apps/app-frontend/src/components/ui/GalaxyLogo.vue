<template>
    <div
        aria-label="Galaxy宝可梦"
        class="flex h-full items-center gap-2 font-extrabold text-[var(--color-text-primary)]"
    >
        <img aria-hidden="true" class="aspect-square h-full object-contain" :src="galaxyLogo" />
        <span v-if="!iconOnly" class="hidden text-sm tracking-wide xl:inline">Galaxy宝可梦</span>
    </div>
</template>

<script setup lang="ts">
import galaxyLogo from '@/assets/galaxy-logo.svg?url'

defineProps<{
    iconOnly?: boolean
}>()
</script>
