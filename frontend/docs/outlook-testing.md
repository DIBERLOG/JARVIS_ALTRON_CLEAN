# Проверка Outlook

Из папки `frontend`:

```powershell
node tests/outlook.cjs
node tests/center-voice.cjs
npm run build
node --conditions=browser tests/outlook-ui.cjs <папка-с-test-only-node_modules>
```

Из корня проекта (для Windows с библиотекой Vosk):

```powershell
$outlookVoskDir = (Resolve-Path -LiteralPath 'lib/windows/amd64').Path
$env:LIB = $outlookVoskDir + ';' + $env:LIB
cargo test -p jarvis-gui tauri_commands::outlook::tests --offline -j2
```

Тесты сценариев используют подменённый вызов backend: реальные письма не читаются и не
отправляются. Проверены подтверждение, его отмена при редактировании, запрет повторной
отправки после неизвестного статуса, выбор по номеру и отсутствие исполнения продиктованной
команды внутри тела письма. Backend отдельно проверяет одноразовость/срок подтверждения,
получателя, Client ID, отсутствие токенов в публичном статусе и фиксированный Graph-host.

Сборка проверяет Svelte/TypeScript. DOM-тест использует тот же отдельный набор зависимостей,
что существующие `news.cjs` и `notes.cjs`: `jsdom`, `@testing-library/dom`, `@testing-library/user-event`.
Проверяются вход, инструкция, внешний браузер, безопасный показ письма, подтверждение и его отмена.
На текущей машине набор находится в `%LOCALAPPDATA%/Temp/jarvis-notes-test-deps`.
После подключения аккаунта вручную
проверьте вход и отмену входа, список писем, поиск, редактирование, отмену подтверждения и
сохранение тестового черновика. Реальную отправку проверяйте только на разрешённый адрес,
после явного подтверждения пользователя. API `202` не подтверждает доставку.
