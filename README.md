# Blep

Desktop-приложение на Tauri и React для установки и запуска GDPS (Geometry Dash Private Servers). Клиент скачивает чистый Geometry Dash, патчит адрес базы под указанный сервер и позволяет запускать установленную сборку.

## Возможности
- Установка сервера по ID с патчем адреса базы и отдельной копией клиента.
- Список найденных локальных GDPS сборок и быстрый выбор.
- Запуск выбранной сборки прямо из приложения (macOS/Windows).
- Фон и метаданные сервера подтягиваются из API.

## Стек
- Tauri 2 + Rust backend (`src-tauri`).
- React 19 + TypeScript + Vite.
- Tailwind CSS 4 и Radix UI компоненты.

## Запуск
- Требования: Bun, Rust toolchain, зависимости для Tauri (Xcode CLT на macOS / MSVC + WebView2 на Windows).
- Установка зависимостей: `bun install`.
- Дев режим: `bun run tauri dev` (поднимет Vite и Tauri).
- Только фронтенд превью: `bun run dev`.
- Сборка приложения: `bun run tauri build`.

## Структура
- `src`: клиент на React.
- `src-tauri`: команды Tauri и патчер Geometry Dash.
- `public/font`: шрифты Stolzl.

## Лицензия
Проект распространяется по лицензии GPL-3.0-or-later, подробности в файле `LICENSE`.
