# Luna на Rust (axum + Leptos)

```
backend/   — сервер: аккаунты (Argon2), история чатов (SQLite), потоковые ответы Claude
frontend/  — интерфейс на Leptos (WASM), стили и вид как в исходной Luna.html
gen.js     — вытаскивает данные (кролик, палитры, документы) из исходной Luna.html в frontend/assets/data.json
```

## Запуск

```bash
rustup target add wasm32-unknown-unknown
cargo install trunk

cd frontend && trunk build --release        # соберёт frontend/dist
cd ../backend
ANTHROPIC_API_KEY=sk-ant-... cargo run --release
# открыть http://localhost:3000
```

Разработка с автоперезагрузкой фронта: `cd frontend && trunk serve` (запросы /api идут на :3000).

## Переменные окружения
| имя | значение по умолчанию |
|---|---|
| `ANTHROPIC_API_KEY` | обязательна |
| `PORT` | 3000 |
| `LUNA_DB` | luna.db |
| `LUNA_STATIC` | ../frontend/dist |
| `LUNA_SECURE` | если задана, cookie получает флаг Secure (нужно за HTTPS) |

## Что изменилось по сравнению с Luna.html
- Ответы ИИ идут через сервер (ключ Anthropic не попадает в браузер). Тариф Lina / Luna / Luna Pro выбирает модель на сервере.
- Аккаунт настоящий: пароль хешируется Argon2, сессия в HttpOnly cookie. Вход через Apple/Google убран (на сервере его нет).
- История чатов хранится на сервере; настройки внешнего вида и персонажа остались в localStorage.
- Писать ИИ можно только после входа. Лимит: 12 запросов в минуту на пользователя.
- Тарифы по-прежнему переключаются свободно (оплата не подключена): см. `set_plan` в backend/src/main.rs.

## Перед публикацией
- Замените `[Название или ФИО оператора]`, `[email для связи]`, `[страна]` в frontend/assets/data.json.
- Модели заданы константой `MODELS` в backend/src/main.rs.
- Вход не защищён от перебора паролей по IP: поставьте за reverse proxy с лимитом или добавьте ограничение.
