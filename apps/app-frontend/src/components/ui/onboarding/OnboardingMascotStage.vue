<script setup lang="ts">
import galaxyLogo from '@/assets/galaxy-logo.svg?url'

defineProps<{
    alt: string
}>()

// Galaxy宝可梦: the onboarding guide uses our logo instead of the Axolotl mascot.
const mascotVisual = galaxyLogo
</script>

<template>
    <img class="onboarding-mascot" :src="mascotVisual" :alt="alt" />
</template>

<style scoped lang="scss">
.onboarding-mascot {
    width: clamp(6rem, 11vw, 9rem);
    height: auto;
    object-fit: contain;
}
</style>
