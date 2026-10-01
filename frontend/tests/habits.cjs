// Run: node --conditions=browser tests/habits.cjs <test dependency directory>
const fs = require('node:fs'), path = require('node:path'), assert = require('node:assert/strict')
const { createRequire, Module } = require('node:module')
const deps = createRequire(path.resolve(process.argv[2], 'package.json'))
const { JSDOM } = deps('jsdom')
const dom = new JSDOM('<!doctype html><body></body>', { url: 'http://localhost' })
for (const name of ['window', 'document', 'navigator', 'HTMLElement', 'Node', 'Event', 'MouseEvent', 'getComputedStyle']) Object.defineProperty(globalThis, name, { value: dom.window[name], configurable: true })
global.confirm = () => true
const { screen, waitFor } = deps('@testing-library/dom')
const userEvent = deps('@testing-library/user-event').default
const ts = require('typescript'), { compile, preprocess } = require('svelte/compiler')
function evaluate(code, filename, overrides) {
    const mod = new Module(filename, module); mod.filename = filename; mod.paths = module.paths
    mod.require = name => overrides[name] || require(name)
    mod._compile(ts.transpileModule(code, { compilerOptions: { module: ts.ModuleKind.CommonJS, target: ts.ScriptTarget.ES2020 } }).outputText, filename)
    return mod.exports
}
async function main() {
    const svelte = await import('svelte')
    const center = evaluate(fs.readFileSync('src/lib/center.ts', 'utf8'), path.resolve('src/lib/center.ts'), { '@tauri-apps/api/core': { invoke: async () => '' } })
    const stats = evaluate(fs.readFileSync('src/lib/habits.ts', 'utf8'), path.resolve('src/lib/habits.ts'), { './center': center })
    let saved = center.defaultHabits()
    const today = center.dayKey(new Date())
    assert.equal(stats.habitStats(saved, today).done, 0)
    assert.equal(stats.habitSeries(saved, today, 7).slice(0, -1).every(point => point.percent === null), true)
    const filename = path.resolve('src/components/CenterHabits.svelte')
    const processed = await preprocess(fs.readFileSync(filename, 'utf8'), require('svelte-preprocess')(), { filename })
    const compiled = compile(processed.code, { filename, generate: 'dom', css: 'external' })
    const Component = evaluate(compiled.js.code, filename, { svelte, '@/lib/center': center, '@/lib/habits': stats }).default
    let component
    const props = () => ({ habits: JSON.parse(JSON.stringify(saved)), onSave: async habits => { saved = JSON.parse(JSON.stringify(habits)); component.$set({ habits: saved }); return true } })
    component = new Component({ target: document.body, props: props() })
    const user = userEvent.setup({ document })
    await user.click(screen.getByRole('button', { name: 'Отметить: Сходил в зал' }))
    await waitFor(() => assert.equal(stats.habitStats(saved, today).done, 1))
    for (let i = 0; i < 8; i++) await user.click(screen.getByRole('button', { name: 'Добавить: Выпил воду' }))
    assert.equal(saved.find(habit => habit.title === 'Выпил воду').entries[today], 8)
    assert.equal(stats.habitStats(saved, today).done, 2)
    await user.click(screen.getByRole('button', { name: /Линия/ }))
    assert.ok(screen.getByRole('img', { name: /Линейный график/ }))
    await user.click(screen.getByRole('button', { name: /Столбцы/ }))
    assert.ok(screen.getByRole('img', { name: /Столбчатый график/ }))
    await user.selectOptions(screen.getByRole('combobox', { name: 'Период статистики' }), '30')
    component.$destroy()
    component = new Component({ target: document.body, props: props() })
    assert.equal(screen.getByRole('button', { name: 'Отметить: Сходил в зал' }).getAttribute('aria-pressed'), 'true')
    await user.click(screen.getByRole('button', { name: /Своя привычка/ }))
    await user.type(screen.getByRole('textbox', { name: 'Название привычки' }), 'Новые слова')
    await user.click(screen.getByRole('button', { name: 'Создать привычку' }))
    await screen.findByRole('button', { name: 'Отметить: Новые слова' })
    const old = { ...saved[0], createdOn: '2026-01-01', archivedOn: '2026-01-03', entries: { '2026-01-02': 1 } }
    assert.equal(stats.habitStats([old], '2026-01-02').done, 1)
    assert.equal(stats.habitStats([old], '2026-01-03').total, 0)
    component.$destroy()
    console.log('PASS: daily marks, water goal, real statistics, chart switching, custom habit, restored saved data and archived history')
}
main().catch(error => { console.error(error); process.exitCode = 1 })
