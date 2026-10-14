# TASK-001: Project Scaffolding (Tauri v2 + Rust + Bun/React)

- **Ticket ID**: `TASK-001`
- **Status**: `TODO`
- **Dependencies**: None
- **Target Files**:
  - `package.json`
  - `tsconfig.json`
  - `vite.config.ts`
  - `tailwind.config.js`
  - `src-tauri/Cargo.toml`
  - `src-tauri/tauri.conf.json`
  - `src-tauri/src/main.rs`
  - `src/main.tsx`
  - `src/index.css`

---

## 1. Objective

Set up the foundational workspace for **MiniStation** using Tauri v2, Rust 1.95, and React 19 / TypeScript / Vite via Bun. Establish the build pipeline so running the dev command launches a native desktop window with hot-reloading and Rust backend bindings ready.

---

## 2. Implementation Steps

1. **Initialize Frontend with Bun**:
   - Create `package.json` with `@tauri-apps/api`, `@tauri-apps/plugin-shell`, `react`, `react-dom`, `lucide-react`, `tailwindcss`, `vite`.
   - Configure `vite.config.ts` with React plugin and Tauri port options (`clearScreen: false`, port 1420).
   - Configure Tailwind CSS with dark slate / Akai color tokens.

2. **Initialize Tauri v2 Rust Backend**:
   - Create `src-tauri/Cargo.toml` with `tauri = "2"`, `serde`, `serde_json`, `tokio`.
   - Create `src-tauri/tauri.conf.json` configured for window size (1120x760 min), title "MiniStation", dark titlebar, and permissions.
   - Scaffold a minimal `src-tauri/src/main.rs` with `tauri::Builder::default().run(...)`.

3. **Verify Dev Build**:
   - Ensure `bun install` completes cleanly.
   - Ensure `cargo check` inside `src-tauri` succeeds.

---

## 3. Acceptance Criteria

- [ ] `bun run build` succeeds without TypeScript or bundling errors.
- [ ] `cargo check --manifest-path src-tauri/Cargo.toml` compiles cleanly without warnings or errors.
- [ ] A baseline window can be launched displaying the initialized React DOM.
