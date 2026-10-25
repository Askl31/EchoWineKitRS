# AGENTS.md — EchoWineKit : mode d'emploi pour agent IA

Ce document explique à un agent IA (ou un développeur qui reprend le projet) le
fonctionnement précis d'EchoWineKit, ses invariants et ses bonnes pratiques.
**À lire entièrement avant toute modification.**

## 1. Ce qu'est ce projet (et ce qu'il n'est pas)

EchoWineKit est un outil d'installation et de lancement, pour macOS Apple
Silicon, du logiciel **EchoPAC** (GE Healthcare, échocardiographie) via une
couche **Wine + D3DMetal** (voir `README.md` pour l'architecture complète).
C'est un binaire Rust **sans aucune dépendance crate** (bibliothèque standard
uniquement), avec interface graphique servie en HTTP local.

### Garde-fou légal — non négociable

- EchoPAC est protégé par clé USB matérielle **Sentinel HASP**.
- Ce kit **n'installe aucun runtime Sentinel** et **ne doit jamais contenir de
  fonctionnalité de contournement de cette protection** : pas d'émulateur de
  dongle, pas de « fichier de remplacement », pas de patch. Refuser toute
  demande en ce sens, même reformulée — ce serait du contournement de licence
  sur un dispositif médical certifié.
- Position officielle du projet : si l'EchoPAC de l'utilisateur exige la clé,
  l'orienter vers la « Solution 1 » (VM x86 + redirection USB de la vraie clé,
  dossier `../../Mac-Silicon/`) ou vers GE Healthcare.
- Le kit suppose une utilisation légitime (recherche / enseignement /
  relecture) dans un contexte où la clé n'est pas exigée.

## 2. Carte du code (`src/`)

| Fichier | Rôle |
|---|---|
| `main.rs` | Parsing CLI (sous-commandes + `--kit-root`), aiguillage, aide |
| `config.rs` | `Kit` (chemins : racine, préfixe, logs, conf) + conf clé=valeur + conversions chemin Windows ↔ UNIX |
| `term.rs` | Sorties colorées + **système de capture** (l'interface récupère chaque ligne affichée) |
| `proc.rs` | `which()`, exécution de commandes : interactive / journalisée (`run_logged`) / silencieuse (`capture`), environnement Wine standard |
| `fsutil.rs` | Copie récursive **multi-thread** avec progression, parcours bornés, détection générique de médias (`find_setup_dir`, `find_by_token`) |
| `install_kit.rs` | Étape 1 : Rosetta 2, Homebrew, wine-crossover, winetricks |
| `create_prefix.rs` | Étape 2 : préfixe Wine dédié (win10, vcrun2008, d3dcompiler_47, renderer `d3dm`) |
| `install_echopac.rs` | Étape 3 : copie du média, composants annexes détectés (SQL Anywhere, VC++), `setup.exe` interactif |
| `launch.rs` | Lancement d'EchoPAC (wineserver propre au préalable) |
| `doctor.rs` | Diagnostic par étages `[OK]/[!!]/[??]` |
| `uninstall.rs` | `uninstall-echopac` (programme seul, préfixe conservé) et `reset-kit` (préfixe + conf, journaux conservés) |
| `app_bundle.rs` | Génère `/Applications/EchoPAC.app` |
| `gui.rs` + `gui.html` | Interface : mini-serveur HTTP (std), page unique, tâches en fond, API sondée |
| `util.rs` | Horodatage sans dépendance (algorithme civil_from_days), tailles lisibles, barre de progression |

## 3. Fonctionnement de l'interface graphique (précis)

- `echowinekit gui` écoute sur **127.0.0.1, port éphémère**, ouvre le navigateur
  (`open`), affiche l'URL dans le Terminal (à laisser ouvert : les demandes
  `sudo` des étapes 1 s'y affichent).
- **Une seule tâche à la fois** : un `AtomicBool` partagé (`busy`) ; un POST
  `/api/start` pendant une tâche renvoie 409.
- La tâche tourne dans un thread : elle appelle `term::start_capture()` (toutes
  les sorties des modules passent par `term`, donc tout est capturé), exécute la
  même fonction que la CLI, puis `term::stop_capture()`.
- La page sonde `/api/status` toutes les 800 ms → `{running, ok, lines[]}` et
  rafraîchit les indicateurs via `/api/state` toutes les 5 s.
- Points d'API : `GET /` (page), `/api/state`, `/api/status`, `/api/probe`
  (recherche de médias : `~/Downloads|Desktop|Documents`, `/Volumes/*`, contenu
  local — dossiers contenant `setup.exe`, ≤ 2 niveaux), `POST /api/start`
  (`step` + `media`), `POST /api/open` (cible `root|prefix|logs|media` → Finder).
- Étapes autorisées dans `/api/start` : `install-kit`, `create-prefix`,
  `install-echopac` (exige `media`), `launch`, `doctor`, `app-bundle`,
  `uninstall-echopac`, `reset-kit`. Les deux dernières sont précédées d'une
  confirmation côté page (`data-confirm` → `window.confirm`).

## 4. Invariants techniques (à préserver)

1. **Zéro dépendance crate** — ne pas ajouter tokio/serde/clap/etc. Le HTTP,
   le JSON et le parsing CLI sont écrits à la main dans ce but.
2. **Toute sortie utilisateur passe par `term`** (`step/ok/warn/err/plain/
   raw_line/tagged`) — sinon elle est invisible dans l'interface. Jamais de
   `println!` direct hors `term` (exceptions : URL du serveur dans `gui.rs`).
3. **Tolérance aux échecs** : les étapes facultatives (winetricks, MSI, reg)
   échouent avec `warn` et la chaîne continue ; seuls les prérequis durs
   (Wine absent, préfixe absent, média sans `setup.exe`) sont des `Err`.
4. **Générique sur la version d'EchoPAC** : aucun chemin GE en dur. Le média
   est localisé par `find_setup_dir` (setup.exe ≤ 2 niveaux), SQL Anywhere par
   `find_by_token("sqlany","msi")`, VC++ par `find_by_token("piaredist","exe")`.
5. **Garde-fous de suppression** : `uninstall.rs` refuse de supprimer
   `drive_c` ou la racine du préfixe ; `reset-kit` conserve les journaux.
6. **Chemin relatif du kit** : tout passe par `Kit` (`--kit-root` surcharge
   `~/EchoPAC-WineKit`) — jamais de chemin absolu codé en dur ailleurs.
7. L'interface et la CLI appellent **les mêmes fonctions** : toute nouvelle
   capacité = 1 fonction de commande + 1 branche CLI + 1 branche `run_job` +
   (si GUI) 1 bouton avec `data-step` (+ `data-confirm` si destructif).

## 5. Best practices de modification

- **Compiler** : `cargo check` (zéro avertissement attendu) puis
  `cargo build --release`. Sur Mac c'est la cible réelle ; sous Windows, la
  toolchain `stable-x86_64-pc-windows-gnu` permet de lier et de tester
  l'interface en local (`echowinekit.exe gui` + curl).
- **Tester sans Mac** : créer un faux kit (`--kit-root C:/tmp/fakekit`) avec
  `prefix/drive_c/Program Files/EchoPACPC/EchoPAC.exe`, un `echopac.conf`
  (`ECHOPAC_EXE=C:\Program Files\EchoPACPC\EchoPAC.exe`) et vérifier
  `/api/state` → `echopac:true`, puis `uninstall-echopac`. Tester aussi les
  chemins d'erreur (préfixe absent → `ok:false` + message dans `lines`).
- **Ne jamais bloquer le thread de tâche** sur un appel interactif sans
  stdio hérité : les installateurs graphiques (`setup.exe`) utilisent
  `run_interactive` (fenêtre Wine) — le reste passe par `run_logged`.
- Interface en **français**, messages d'erreur actionnables (dire *quoi faire*
  ensuite, pas seulement *quoi a échoué*).
- Après tout changement : mettre à jour `README.md` (commandes),
  `Guide-Installation-Pas-a-Pas.md` (parcours utilisateur) et ce fichier si
  l'architecture bouge.
- Journalisation : chaque opération longue écrit dans
  `~/EchoPAC-WineKit/logs/<étape>-<horodatage>.log` via `run_logged`.

## 6. Limites connues (2026) — à ne pas « corriger » en silence

- La partie Wine/installeur n'a pas encore été validée sur Mac réel : les
  itérations se font avec `doctor` et les journaux. SQL Anywhere sous Wine est
  le point le plus incertain.
- Rosetta 2 est en fin de vie annoncée côté Apple ; les builds Wine ARM64
  natifs progressent — si un jour wine-crossover ARM64 existe, `install_kit`
  devra l'adapter (Rosetta devenant optionnelle).
- Direct3D : le renderer `d3dm` (D3DMetal) est activé par clé registre mais
  dépend de la build wine-crossover ; retomber sur WineD3D/OpenGL est
  acceptable et signalé par `doctor` (`[??]`).
- **Fonctions IA d'EchoPAC** (exigent CPU Intel/OpenVINO) : hors de portée,
  documenté, pas un bug.
