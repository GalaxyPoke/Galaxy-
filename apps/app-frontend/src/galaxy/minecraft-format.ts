/**
 * Minecraft legacy formatting codes ('&x' as used by server plugins, or '§x').
 * The text is split into plain segments; components render them with Vue text
 * interpolation plus a class, never as HTML, so announcement text from the
 * server cannot inject markup.
 *
 * Supported: 0-9 a-f colours, l bold, o italic, n underline, m strikethrough,
 * r reset. k (obfuscated) is ignored and the text stays readable. As in
 * Minecraft, a colour code also clears the format flags set before it.
 */
export type MinecraftColorCode =
    | '0'
    | '1'
    | '2'
    | '3'
    | '4'
    | '5'
    | '6'
    | '7'
    | '8'
    | '9'
    | 'a'
    | 'b'
    | 'c'
    | 'd'
    | 'e'
    | 'f'

export interface MinecraftTextSegment {
    text: string
    color: MinecraftColorCode | null
    bold: boolean
    italic: boolean
    underline: boolean
    strikethrough: boolean
}

const CODE_PATTERN = /[&§]([0-9a-fk-or])/gi

export function parseMinecraftText(input: string): MinecraftTextSegment[] {
    const segments: MinecraftTextSegment[] = []
    let state: Omit<MinecraftTextSegment, 'text'> = {
        color: null,
        bold: false,
        italic: false,
        underline: false,
        strikethrough: false,
    }
    let last = 0

    const push = (text: string) => {
        if (!text) return
        const previous = segments[segments.length - 1]
        if (
            previous &&
            previous.color === state.color &&
            previous.bold === state.bold &&
            previous.italic === state.italic &&
            previous.underline === state.underline &&
            previous.strikethrough === state.strikethrough
        ) {
            previous.text += text
        } else {
            segments.push({ text, ...state })
        }
    }

    for (const match of input.matchAll(CODE_PATTERN)) {
        const index = match.index ?? 0
        push(input.slice(last, index))
        last = index + match[0].length
        const code = match[1].toLowerCase()
        if (/[0-9a-f]/.test(code)) {
            state = {
                color: code as MinecraftColorCode,
                bold: false,
                italic: false,
                underline: false,
                strikethrough: false,
            }
        } else if (code === 'r') {
            state = {
                color: null,
                bold: false,
                italic: false,
                underline: false,
                strikethrough: false,
            }
        } else if (code === 'l') {
            state = { ...state, bold: true }
        } else if (code === 'o') {
            state = { ...state, italic: true }
        } else if (code === 'n') {
            state = { ...state, underline: true }
        } else if (code === 'm') {
            state = { ...state, strikethrough: true }
        }
        // 'k' (obfuscated) is dropped on purpose.
    }
    push(input.slice(last))
    return segments
}

/** The text without any formatting codes (for titles, tooltips, search). */
export function stripMinecraftCodes(input: string): string {
    return input.replace(CODE_PATTERN, '')
}
