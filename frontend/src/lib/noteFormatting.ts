import type { ChecklistItem } from "./center"

export function textToChecklist(text: string, createId: () => string): ChecklistItem[] {
    return text.split(/\r?\n/).filter(line => line.trim()).map(line => ({
        id: createId(),
        done: /^\s*(?:-\s*\[[xX]\]|☑)/.test(line),
        text: line.replace(/^\s*(?:-\s*\[[ xX]\]\s*|[☑☐]\s*|[-*•]\s+|\d+\.\s+|#{1,3}\s+)/, "").trim()
    }))
}

export function checklistToText(items: ChecklistItem[]): string {
    return items.map(item => `- [${item.done ? "x" : " "}] ${item.text}`).join("\n")
}

// Escape all user input before adding only known formatting tags. No raw HTML or links.
export function renderNote(text: string, formatted: boolean): string {
    const escaped = text.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;").replace(/"/g, "&quot;").replace(/'/g, "&#39;")
    if (!formatted) return escaped.replace(/\n/g, "<br>")
    function inline(line: string) {
        return line.replace(/\*\*([^*\n]+)\*\*/g, "<strong>$1</strong>")
            .replace(/\*([^*\n]+)\*/g, "<em>$1</em>")
            .replace(/~~([^~\n]+)~~/g, "<s>$1</s>")
    }
    return escaped.split(/\r?\n/).map(line => {
        const heading = /^(#{1,3})\s+(.*)$/.exec(line)
        if (heading) return `<h${heading[1].length + 1}>${inline(heading[2])}</h${heading[1].length + 1}>`
        const check = /^-\s*\[([ xX])\]\s+(.*)$/.exec(line)
        if (check) return `<p>${check[1].toLowerCase() === "x" ? "☑" : "☐"} ${inline(check[2])}</p>`
        if (/^[-*]\s+/.test(line)) return `<p>• ${inline(line.replace(/^[-*]\s+/, ""))}</p>`
        return `<p>${inline(line) || "<br>"}</p>`
    }).join("")
}

export function formatSelection(text: string, start: number, end: number, style: string) {
    const selected = text.slice(start, end)
    const wrappers: Record<string, string> = { bold: "**", italic: "*", strike: "~~" }
    if (wrappers[style]) {
        const marker = wrappers[style], content = selected || "текст"
        return { text: text.slice(0, start) + marker + content + marker + text.slice(end), start: start + marker.length, end: start + marker.length + content.length }
    }
    const lineStart = text.lastIndexOf("\n", start - 1) + 1
    const nextNewline = text.indexOf("\n", end)
    const lineEnd = nextNewline < 0 ? text.length : nextNewline
    const prefix = style === "heading" ? "## " : "- "
    const content = text.slice(lineStart, lineEnd).split("\n").map(line => prefix + line).join("\n")
    return { text: text.slice(0, lineStart) + content + text.slice(lineEnd), start: lineStart, end: lineStart + content.length }
}
