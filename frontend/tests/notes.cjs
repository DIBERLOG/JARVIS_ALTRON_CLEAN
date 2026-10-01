// Run: node --conditions=browser tests/notes.cjs <directory containing test-only node_modules>
const fs = require('node:fs')
const path = require('node:path')
const assert = require('node:assert/strict')
const { createRequire, Module } = require('node:module')
const deps = createRequire(path.resolve(process.argv[2], 'package.json'))
const { JSDOM } = deps('jsdom')
const dom = new JSDOM('<!doctype html><html><body></body></html>', { url: 'http://localhost' })
for (const name of ['window', 'document', 'navigator', 'HTMLElement', 'HTMLAnchorElement', 'Node', 'Event', 'MouseEvent', 'getComputedStyle', 'DOMParser', 'MutationObserver', 'File']) {
    Object.defineProperty(globalThis, name, { value: dom.window[name], configurable: true })
}
window.confirm = () => true
window.scrollBy = () => {}
global.requestAnimationFrame = fn => setTimeout(fn, 0)
global.cancelAnimationFrame = clearTimeout
window.requestAnimationFrame = global.requestAnimationFrame
window.cancelAnimationFrame = clearTimeout
const rect = { top: 0, bottom: 10, left: 0, right: 10, width: 10, height: 10 }
dom.window.Range.prototype.getClientRects = () => [rect]
dom.window.Range.prototype.getBoundingClientRect = () => rect
dom.window.HTMLElement.prototype.getClientRects = () => [rect]
document.elementFromPoint = () => document.body
const { screen, waitFor } = deps('@testing-library/dom')
const userEvent = deps('@testing-library/user-event').default
const ts = require('typescript')
const { compile, preprocess } = require('svelte/compiler')
const sveltePreprocess = require('svelte-preprocess')
function evaluate(code, filename, overrides = {}) {
    const mod = new Module(filename, module)
    mod.filename = filename
    mod.paths = module.paths
    mod.require = name => overrides[name] || require(name)
    mod._compile(ts.transpileModule(code, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText, filename)
    return mod.exports
}
async function main() {
    const svelte = await import('svelte')
    const formatting = evaluate(fs.readFileSync('src/lib/noteFormatting.ts', 'utf8'), path.resolve('src/lib/noteFormatting.ts'))
    assert.equal(formatting.renderNote('<img src=x onerror=alert(1)>', true).includes('<img'), false)
    assert.match(formatting.renderNote('**Текст**', true), /<strong>Текст<\/strong>/)
    assert.equal(formatting.renderNote('**старый текст**', false), '**старый текст**')
    const rich = evaluate(fs.readFileSync('src/lib/richNotes.ts', 'utf8'), path.resolve('src/lib/richNotes.ts'), { dompurify: { default: require('dompurify') } })
    assert.equal(rich.sanitizeNoteHtml('<script>alert(1)</script><img src="https://tracker.test/a"><a href="javascript:alert(1)">bad</a>').includes('<script'), false)
    assert.equal(rich.sanitizeNoteHtml('<img src="https://tracker.test/a">').includes('<img'), false)
    await assert.rejects(rich.readImage(new File(['bad'], 'unsafe.svg', { type: 'image/svg+xml' })))
    const png = 'data:image/png;base64,iVBORw0KGgoAAAANSUhEUgAAAAEAAAABCAYAAAAfFcSJAAAADUlEQVQIHWP4z8DwHwAFgAI/ScLbtAAAAABJRU5ErkJggg=='
    const richFilename = path.resolve('src/components/RichNoteEditor.svelte')
    const richProcessed = await preprocess(fs.readFileSync(richFilename, 'utf8'), sveltePreprocess(), { filename: richFilename })
    const richCompiled = compile(richProcessed.code, { filename: richFilename, generate: 'dom', css: 'external' })
    const RichComponent = evaluate(richCompiled.js.code, richFilename, { svelte, '@/lib/richNotes': { ...rich, readImage: async () => png } }).default
    const filename = path.resolve('src/components/CenterNotes.svelte')
    const processed = await preprocess(fs.readFileSync(filename, 'utf8'), sveltePreprocess(), { filename })
    const compiled = compile(processed.code, { filename, generate: 'dom', css: 'external' })
    let counter = 0, saved
    const Component = evaluate(compiled.js.code, filename, { svelte, './RichNoteEditor.svelte': { default: RichComponent }, '@/lib/richNotes': rich, '@/lib/center': { makeId: () => `test-${++counter}` }, '@/lib/noteFormatting': formatting }).default
    const notes = [{ id: 'legacy', title: 'Старая заметка', text: 'Существующий текст', updatedAt: '2026-10-01T00:00:00Z' }]
    const component = new Component({ target: document.body, props: { notes, onSave: async note => { saved = JSON.parse(JSON.stringify(note)); notes.push(saved); component.$set({ notes: [...notes] }); return true }, onDelete: async () => true } })
    const user = userEvent.setup({ document })
    await user.click(screen.getByRole('button', { name: /Старая заметка/ }))
    assert.equal(screen.getByRole('textbox', { name: 'Текст заметки' }).textContent, 'Существующий текст')
    await user.click(screen.getByRole('button', { name: /Новая/ }))
    await user.type(screen.getByRole('textbox', { name: 'Заголовок заметки' }), 'Покупки')
    await user.click(screen.getByRole('button', { name: /^☑ Чек-лист$/ }))
    await user.click(screen.getByRole('button', { name: /Добавить пункт/ }))
    await user.type(screen.getByRole('textbox', { name: 'Текст пункта 1' }), 'Молоко')
    await user.click(screen.getByRole('checkbox', { name: /Выполнено: Молоко/ }))
    await user.click(screen.getByRole('button', { name: /Создать заметку/ }))
    await waitFor(() => assert.equal(saved?.type, 'checklist'))
    assert.equal(saved.items[0].done, true)
    const checklistId = saved.id
    await user.click(screen.getByRole('button', { name: /Новая/ }))
    await user.click(screen.getByRole('button', { name: /Покупки/ }))
    assert.equal(screen.getByRole('checkbox', { name: /Выполнено: Молоко/ }).checked, true)
    await user.click(screen.getByRole('button', { name: /Обычная/ }))
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).textContent, /Молоко/)
    await user.click(screen.getByRole('button', { name: /^☑ Чек-лист$/ }))
    assert.equal(screen.getByRole('checkbox', { name: /Выполнено: Молоко/ }).checked, true)
    await user.click(screen.getByRole('button', { name: /Новая/ }))
    await user.type(screen.getByRole('textbox', { name: 'Заголовок заметки' }), 'Форматирование')
    await user.click(screen.getByRole('button', { name: 'Жирный текст' }))
    await user.type(screen.getByRole('textbox', { name: 'Текст заметки' }), 'Пример')
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).innerHTML, /<strong>/)
    await user.click(screen.getByRole('button', { name: 'Подчёркнутый текст' }))
    await user.type(screen.getByRole('textbox', { name: 'Текст заметки' }), 'Подчеркнуто')
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).innerHTML, /<u>/)
    await user.selectOptions(screen.getByRole('combobox', { name: 'Стиль абзаца' }), '1')
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).innerHTML, /<h1>/)
    await user.selectOptions(screen.getByRole('combobox', { name: 'Размер текста' }), '24px')
    await user.click(screen.getByRole('button', { name: 'По центру' }))
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).innerHTML, /text-align: center/)
    await user.click(screen.getByRole('button', { name: 'Отменить действие' }))
    await user.click(screen.getByRole('button', { name: 'Повторить действие' }))
    assert.match(screen.getByRole('textbox', { name: 'Текст заметки' }).innerHTML, /text-align: center/)
    const imageFile = new File(['image bytes'], 'test.png', { type: 'image/png' })
    const clipboard = { items: [{ kind: 'file', type: 'image/png', getAsFile: () => imageFile }], files: [imageFile], types: ['Files'], getData: () => '' }
    await user.paste(clipboard)
    await screen.findByRole('img', { name: 'test.png' })
    await user.click(screen.getByRole('button', { name: /Создать заметку/ }))
    await waitFor(() => assert.equal(saved?.format, 'rich'))
    assert.match(saved.html, /data:image\/png;base64/)
    await user.click(screen.getByRole('button', { name: /Новая/ }))
    await user.click(screen.getByRole('button', { name: /Форматирование/ }))
    await screen.findByRole('img', { name: 'test.png' })
    assert.notEqual(saved.id, checklistId)
    assert.equal(notes[0].text, 'Существующий текст')
    component.$destroy()
    console.log('PASS: old notes preserved; checklist save and conversion; visual bold/underline; clipboard image paste and saved reopen; HTML sanitization (image decode mocked in jsdom)')
}
main().catch(error => { console.error(error); process.exitCode = 1 })
