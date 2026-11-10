import { vanillaDefaultSkinForName } from '@/galaxy/default-skin'
import { generatePlayerHeadBlob } from '@/helpers/rendering/batch-skin-renderer'
import type { Skin } from '@/helpers/skins'
import { get_available_skins, get_normalized_skin_texture } from '@/helpers/skins'

/**
 * Player heads for the header chip, the play dock, the account list and the
 * name dialog.
 *
 * These return PNG data URLs on purpose. The shared head cache in
 * batch-skin-renderer hands out blob: URLs and revokes every one the Skins
 * page is not currently showing, right after rendering. Anything outside that
 * page that kept such a URL ended up with a dead image, which the Avatar
 * component then replaces with its grey cube.
 */
const headCache = new Map<string, Promise<string>>()

function blobToDataUrl(blob: Blob): Promise<string> {
    return new Promise((resolve, reject) => {
        const reader = new FileReader()
        reader.onload = () => resolve(String(reader.result))
        reader.onerror = () => reject(reader.error ?? new Error('Failed to read head image'))
        reader.readAsDataURL(blob)
    })
}

export function getPlayerHeadDataUrl(skin: Skin): Promise<string> {
    const key = skin.texture_key
    let head = headCache.get(key)
    if (!head) {
        head = (async () => {
            const texture = await get_normalized_skin_texture(skin)
            return await blobToDataUrl(await generatePlayerHeadBlob(texture))
        })()
        headCache.set(key, head)
        head.catch(() => {
            if (headCache.get(key) === head) headCache.delete(key)
        })
    }
    return head
}

let defaultSkins: Promise<Skin[]> | null = null

function bundledDefaultSkins(): Promise<Skin[]> {
    defaultSkins ??= get_available_skins(true)
        .then((skins) => skins.filter((skin) => skin.source === 'default'))
        .catch((error) => {
            defaultSkins = null
            throw error
        })
    return defaultSkins
}

/**
 * Head of the vanilla default skin `name` wears in game (offline UUID), for
 * other players such as the online list. Null when it cannot be rendered.
 */
export async function getDefaultHeadForName(name: string): Promise<string | null> {
    try {
        const skin = findVanillaDefaultSkin(await bundledDefaultSkins(), name)
        return skin ? await getPlayerHeadDataUrl(skin) : null
    } catch (error) {
        console.warn('[galaxy] failed to render a default head for', name, error)
        return null
    }
}

/** The bundled vanilla default skin that `name` wears in game when it has no skin of its own. */
export function findVanillaDefaultSkin(skins: readonly Skin[], name: string): Skin | null {
    const choice = vanillaDefaultSkinForName(name)
    return (
        skins.find(
            (skin) =>
                skin.source === 'default' &&
                skin.name === choice.name &&
                skin.variant === choice.variant,
        ) ?? null
    )
}
