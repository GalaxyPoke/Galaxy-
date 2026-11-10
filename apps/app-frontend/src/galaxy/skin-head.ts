import steveSkinTexture from '@/assets/skins/steve.png?inline'

/** Face + hat layer of a 64x64 skin texture as an SVG data URL. */
export function createSkinHeadDataUrl(textureUrl: string) {
    const escapedTextureUrl = textureUrl
        .replaceAll('&', '&amp;')
        .replaceAll('"', '&quot;')
        .replaceAll('<', '&lt;')
        .replaceAll('>', '&gt;')
    const svg = `<svg xmlns="http://www.w3.org/2000/svg" width="64" height="64" viewBox="0 0 8 8" shape-rendering="crispEdges"><image href="${escapedTextureUrl}" x="-8" y="-8" width="64" height="64" style="image-rendering:pixelated"/><image href="${escapedTextureUrl}" x="-40" y="-8" width="64" height="64" style="image-rendering:pixelated"/></svg>`

    return `data:image/svg+xml;charset=utf-8,${encodeURIComponent(svg)}`
}

/** Default Steve head, used before a player has a skin. */
export const defaultSteveHeadUrl = createSkinHeadDataUrl(steveSkinTexture)
