# Trade Copier — Production Readiness Roadmap

## Kritiek (fix voor release)

- [x] **Router binden op `127.0.0.1` i.p.v. `127.0.0.1`**
  `src-tauri/src/router.rs:11` — Router luistert op alle interfaces, waardoor elke machine op het netwerk verbinding kan maken. Bind op `127.0.0.1:5000` voor enkel lokale connecties.

- [x] **Graceful shutdown implementeren**
  `src-tauri/src/main.rs:164` — Bij het sluiten van het venster worden workers, Wine processen en TCP listeners niet opgeruimd. Implementeer cleanup in de `on_window_event` handler om orphaned processen en bezette poorten te voorkomen.

- [x] **`println!`/`eprintln!` vervangen door `log` crate**
  `router.rs`, `worker.rs`, `worker_manager.rs`, `installer/*.rs`, `ea_sync.rs`, `tauri_commands.rs` gebruiken nog steeds raw `println!`/`eprintln!`. Op Windows met `windows_subsystem = "windows"` gaat deze output verloren. Gebruik consistent `log::info!`, `log::error!`, etc.

- [ ] **`BrowserRouter` vervangen door `HashRouter`**
  `src/App.tsx` — Tauri serveert content via een custom protocol (`tauri://`), niet via een HTTP server. `BrowserRouter` kan routing-problemen veroorzaken in production builds.

- [ ] **Windows force-start: alleen specifieke MT5 instance stoppen**
  `src-tauri/src/installer/windows.rs:159` — `taskkill /F /IM terminal64.exe` doodt alle MT5 instances. Gebruik process ID filtering om alleen de specifieke instance te stoppen.

## Medium (kwaliteitsverbeteringen)

- [ ] **Tauri v2 migratie evalueren**
  De app gebruikt Tauri v1. Tauri v2 (stable sinds oktober 2024) biedt betere app store signing, verbeterd security model en plugin architectuur.

- [ ] **Test coverage uitbreiden**
  Alleen `types.rs` heeft unit tests (6 tests). Voeg tests toe voor database, router, worker en installer modules.

- [ ] **Tauri updater activeren voor GitHub Releases**
  `src-tauri/tauri.conf.json` heeft `"updater": { "active": false }`. Voor directe downloads (buiten app stores) krijgen gebruikers geen auto-updates.

- [ ] **React Query client configureren**
  `src/App.tsx` — QueryClient gebruikt defaults. Configureer `staleTime`, `retry` en `gcTime` voor optimale performance bij 2-seconde polling.

- [ ] **`lovable-tagger` verwijderen uit devDependencies**
  `package.json:81` — Development tool dat niet thuishoort in een productie project.

- [ ] **Root `Cargo.toml` opruimen**
  Root `Cargo.toml` bevat `axum`, `tower-http` en verwijst naar `src/main.rs`. Dit is een overblijfsel van vóór de Tauri-migratie.

- [ ] **Ongebruikte `serde_yaml` dependency verwijderen**
  Staat in zowel root als `src-tauri/Cargo.toml` maar wordt nergens gebruikt. Vergroot onnodig de binary.

- [ ] **`Cargo.lock` uit `.gitignore` halen**
  Voor applicaties (niet libraries) moet `Cargo.lock` gecommit worden voor reproduceerbare builds.

## Laag (nice to have)

- [ ] **React Error Boundary toevoegen**
  Als een component crasht, toont de hele app een wit scherm. Voeg een Error Boundary toe met een gebruiksvriendelijke foutmelding.

- [ ] **Database retention policy implementeren**
  `worker_errors` en `trades` tabellen groeien onbeperkt. Voeg een limiet toe (bijv. max 10.000 records of 30 dagen).

- [ ] **Windows installer pad verplaatsen naar `%LOCALAPPDATA%`**
  `src-tauri/src/installer/windows.rs:263` — Hardcoded pad `C:\MT5-{name}` volgt niet de Windows conventies.

- [ ] **Temp file leak fixen in installer**
  `src-tauri/src/installer/common.rs:52` — `std::mem::forget(temp_file)` lekt de TempFile handle. Gebruik een expliciete cleanup strategie.

- [ ] **i18n support toevoegen**
  De UI is Engels-only. Voor brede app store distributie is lokalisatie wenselijk.

- [ ] **Rate limiting op TCP router**
  Een lokaal proces kan de router overspoelen met trades. Basic rate limiting is prudent voor een financiële applicatie.

- [ ] **FTMO API URL configureerbaar maken**
  `src-tauri/src/tauri_commands.rs:341` — Hardcoded URL naar `gw2.ftmo.com`. Maak configureerbaar en documenteer in het privacy beleid.
