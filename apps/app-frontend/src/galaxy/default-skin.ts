/**
 * Which vanilla default skin a player gets when they have no skin of their own.
 *
 * The Galaxy宝可梦 server runs in offline mode, so a player's UUID is
 * `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`, and the game picks the
 * default skin with `DefaultPlayerSkin.get(uuid)`:
 * `DEFAULT_SKINS[Math.floorMod(uuid.hashCode(), 18)]`, where `DEFAULT_SKINS`
 * is nine slim skins followed by the same nine wide ones (checked against the
 * 1.21.1 client jar). Reproducing that lets the launcher show the head other
 * players actually see in game, the way PCL does when you type a name.
 *
 * Kept free of app imports so it can be checked with plain Node.
 */

export const VANILLA_DEFAULT_SKIN_NAMES = [
    'Alex',
    'Ari',
    'Efe',
    'Kai',
    'Makena',
    'Noor',
    'Steve',
    'Sunny',
    'Zuri',
] as const

export type VanillaDefaultSkinName = (typeof VANILLA_DEFAULT_SKIN_NAMES)[number]

export interface VanillaDefaultSkin {
    name: VanillaDefaultSkinName
    variant: 'SLIM' | 'CLASSIC'
}

const MD5_SHIFTS = [
    7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 7, 12, 17, 22, 5, 9, 14, 20, 5, 9, 14, 20, 5, 9,
    14, 20, 5, 9, 14, 20, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 4, 11, 16, 23, 6, 10, 15, 21,
    6, 10, 15, 21, 6, 10, 15, 21, 6, 10, 15, 21,
]
const MD5_CONSTANTS = Array.from(
    { length: 64 },
    (_, index) => Math.floor(Math.abs(Math.sin(index + 1)) * 2 ** 32) >>> 0,
)

/** RFC 1321 MD5. Web Crypto has no MD5, and this is the only place that needs it. */
function md5(message: Uint8Array): Uint8Array {
    const paddedLength = (((message.length + 8) >>> 6) + 1) << 6
    const buffer = new Uint8Array(paddedLength)
    buffer.set(message)
    buffer[message.length] = 0x80
    const view = new DataView(buffer.buffer)
    const bitLength = message.length * 8
    view.setUint32(paddedLength - 8, bitLength >>> 0, true)
    view.setUint32(paddedLength - 4, Math.floor(bitLength / 2 ** 32), true)

    let a0 = 0x67452301
    let b0 = 0xefcdab89
    let c0 = 0x98badcfe
    let d0 = 0x10325476
    for (let offset = 0; offset < paddedLength; offset += 64) {
        let a = a0
        let b = b0
        let c = c0
        let d = d0
        for (let round = 0; round < 64; round++) {
            let mixed: number
            let wordIndex: number
            if (round < 16) {
                mixed = (b & c) | (~b & d)
                wordIndex = round
            } else if (round < 32) {
                mixed = (d & b) | (~d & c)
                wordIndex = (5 * round + 1) % 16
            } else if (round < 48) {
                mixed = b ^ c ^ d
                wordIndex = (3 * round + 5) % 16
            } else {
                mixed = c ^ (b | ~d)
                wordIndex = (7 * round) % 16
            }
            const sum =
                (a + mixed + MD5_CONSTANTS[round] + view.getUint32(offset + wordIndex * 4, true)) |
                0
            const shift = MD5_SHIFTS[round]
            a = d
            d = c
            c = b
            b = (b + ((sum << shift) | (sum >>> (32 - shift)))) | 0
        }
        a0 = (a0 + a) | 0
        b0 = (b0 + b) | 0
        c0 = (c0 + c) | 0
        d0 = (d0 + d) | 0
    }

    const digest = new Uint8Array(16)
    const digestView = new DataView(digest.buffer)
    digestView.setUint32(0, a0 >>> 0, true)
    digestView.setUint32(4, b0 >>> 0, true)
    digestView.setUint32(8, c0 >>> 0, true)
    digestView.setUint32(12, d0 >>> 0, true)
    return digest
}

/** The 16 bytes of `UUID.nameUUIDFromBytes("OfflinePlayer:" + name)`. */
export function offlinePlayerUuid(name: string): Uint8Array {
    const uuid = md5(new TextEncoder().encode(`OfflinePlayer:${name}`))
    uuid[6] = (uuid[6] & 0x0f) | 0x30
    uuid[8] = (uuid[8] & 0x3f) | 0x80
    return uuid
}

/** `java.util.UUID#hashCode`: the four 32-bit words XORed together. */
export function javaUuidHashCode(uuid: Uint8Array): number {
    const view = new DataView(uuid.buffer, uuid.byteOffset, 16)
    return view.getInt32(0) ^ view.getInt32(4) ^ view.getInt32(8) ^ view.getInt32(12)
}

export function formatUuid(uuid: Uint8Array): string {
    const hex = Array.from(uuid, (byte) => byte.toString(16).padStart(2, '0')).join('')
    return `${hex.slice(0, 8)}-${hex.slice(8, 12)}-${hex.slice(12, 16)}-${hex.slice(16, 20)}-${hex.slice(20)}`
}

export function vanillaDefaultSkinForUuid(uuid: Uint8Array): VanillaDefaultSkin {
    const count = VANILLA_DEFAULT_SKIN_NAMES.length * 2
    const index = ((javaUuidHashCode(uuid) % count) + count) % count
    return {
        name: VANILLA_DEFAULT_SKIN_NAMES[index % VANILLA_DEFAULT_SKIN_NAMES.length],
        variant: index < VANILLA_DEFAULT_SKIN_NAMES.length ? 'SLIM' : 'CLASSIC',
    }
}

/** The default skin an offline player called `name` has in game. */
export function vanillaDefaultSkinForName(name: string): VanillaDefaultSkin {
    return vanillaDefaultSkinForUuid(offlinePlayerUuid(name))
}
