const trimTrailingSlash = (url: string) => url.replace(/\/$/, '')

/**
 * Galaxy宝可梦 brand values (see GALAXY_CONTRACT.md). The object keeps its
 * upstream name so Axolotl merges stay simple. Axolotl's own website, QQ and
 * sponsor links are intentionally blank: this fork must not send players to
 * the upstream community. `upstreamRepositoryUrl` is only used for the GPL
 * attribution on the About page.
 */
export const AxolotlBrandConfig = Object.freeze({
    productName: 'Galaxy宝可梦',
    shortProductName: 'Galaxy宝可梦',
    website: '',
    repositoryUrl: '',
    upstreamRepositoryUrl: 'https://github.com/Mystic-Stars/Axolotl',
    upstreamModrinthSourceUrl: 'https://github.com/modrinth/code',
    supportUrl: '',
    qqGroupNumber: '',
    qqChannelUrl: '',
    sponsorUrl: '',
    bundleIdentifier: 'com.galaxypokemon.launcher',
    deepLinkScheme: 'galaxypokemon',
    userAgent: (version: string, os: string) => `GalaxyPokemon-Launcher/${version} (${os})`,
    capabilities: Object.freeze({
        publicModrinthApi: true,
        privateModrinthServices: false,
        ghsTelemetry: false,
    }),
})

const siteUrl = trimTrailingSlash(import.meta.env.MODRINTH_URL || 'https://modrinth.com')
const officialLabrinthBaseUrl = trimTrailingSlash(
    import.meta.env.MODRINTH_API_BASE_URL || 'https://api.modrinth.com',
)
type DownloadSourceMode = 'auto' | 'official_only' | 'mirror_preferred' | 'official_preferred'

// The Modrinth API always uses the official source; Modrinth download mirror
// routing is handled by the Rust download layer.
export function setModrinthSourceMode(_sourceMode: DownloadSourceMode) {}

export function setModrinthMirrorEnabled(_enabled: boolean) {}

export function getOfficialLabrinthBaseUrl() {
    return officialLabrinthBaseUrl
}

export function getLabrinthBaseUrl() {
    return officialLabrinthBaseUrl
}

export const config = {
    siteUrl,
    labrinthBaseUrl: getLabrinthBaseUrl,
}
