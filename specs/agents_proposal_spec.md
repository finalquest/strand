# Agents Support for Strand CLI

## Overview

Extiende Strand CLI para soportar la gestión de **agents** con la misma funcionalidad que existe para **skills**.

Un agent es un asistente especializado de IA configurable (para OpenCode, Codex u otros CLIs) distribuido desde un repositorio GitLab centralizado, con el mismo formato de directorio, versionado y flujo de instalación que los skills.

## Goals

- Replicar el 100% de la funcionalidad de skills para agents.
- Mantener backward compatibility: skills sigue siendo el default implícito.
- Permitir configurar skills repo, agents repo, o ambos independientemente.
- Instalación física en `.agents/agents/` con symlinks a los directorios que cada CLI espera.
- `strand install` y `strand sync` operan sobre **todos los artefactos configurados** (skills + agents).
- Los comandos específicos de agents usan el namespace `strand agents <command>`.

## Non-Goals

- Cambiar el comportamiento existente de skills.
- Soportar formato TOML para agents (usamos `.md` con frontmatter YAML, igual que skills).
- Resolver dependencias entre agents.
- Publicación de agents desde Strand CLI.

---

## Consumer Repository Layout (con agents)

```text
repo/
  .strand/
    config.json

  .agents/
    skills/
      react-native-review/
    agents/
      security-auditor/
        AGENT.md

  .opencode/
    agents/
      security-auditor -> ../../.agents/agents/security-auditor

  .codex/
    skills/
      react-native-review -> ../../.agents/skills/react-native-review
    agents/
      security-auditor -> ../../.agents/agents/security-auditor

  .gitignore
```

---

## Configuration (`config.json`)

Agrega `agentsRepo` y `agents` al schema existente. Todo lo relacionado a skills permanece sin cambios.

### Nuevo Schema

```json
{
  "version": 1,
  "targets": {
    "opencode": true,
    "codex": false
  },
  "skillsRepo": {
    "provider": "gitlab",
    "project": "group/common-skills",
    "branch": "main",
    "baseUrl": "https://gitlab.com"
  },
  "agentsRepo": {
    "provider": "gitlab",
    "project": "group/common-agents",
    "branch": "main",
    "baseUrl": "https://gitlab.com"
  },
  "skills": [
    {
      "name": "react-native-review",
      "version": "1.2.0",
      "installedPath": ".agents/skills/react-native-review"
    }
  ],
  "agents": [
    {
      "name": "security-auditor",
      "version": "1.0.0",
      "installedPath": ".agents/agents/security-auditor"
    }
  ]
}
```

### Reglas

- `agentsRepo` usa la **misma estructura** que `skillsRepo`: `provider`, `project`, `branch`, `baseUrl`.
- `agents` usa la **misma estructura** que `skills`: `name`, `version`, `installedPath`.
- Todo el comportamiento de `skillsRepo` y `skills` sigue exactamente igual.
- Es válido tener solo `skillsRepo`, solo `agentsRepo`, o ambos.

---

## Agents Repository Layout

Estructura idéntica a skills, pero bajo el directorio `agents/`:

```text
common-agents/
  agents/
    security-auditor/
      AGENT.md
      assets/

    code-reviewer/
      AGENT.md
      assets/
```

### `AGENT.md`

Mismo formato que `SKILL.md`: frontmatter YAML + contenido markdown.

```markdown
---
name: security-auditor
description: Performs security audits and identifies vulnerabilities
metadata:
  version: 1.0.0
---

You are a security expert. Focus on identifying potential security issues.
```

---

## Commands

### Namespace: skills (default, implícito)

| Comando | Descripción |
|---------|-------------|
| `strand ls` | Lista skills instalados con comparación de versiones. |
| `strand ls-remote` | Lista skills remotos con selección fuzzy e instalación. |
| `strand install` | Instala skills **y** agents según lo configurado en `config.json`. |
| `strand sync` | Sincroniza skills **y** agents según lo configurado. |
| `strand validate` | Valida la estructura de skills instalados. |

### Namespace: agents (`strand agents <subcommand>`)

| Comando | Descripción |
|---------|-------------|
| `strand agents ls` | Lista agents instalados con comparación de versiones. |
| `strand agents ls-remote` | Lista agents remotos con selección fuzzy e instalación. |
| `strand agents install` | Alias explícito para instalar solo agents (no se usa directamente; `strand install` lo cubre). |
| `strand agents sync` | Alias explícito para sincronizar solo agents (no se usa directamente; `strand sync` lo cubre). |
| `strand agents validate` | Valida la estructura de agents instalados. |

### `strand install` (modificado)

1. Lee `config.json`.
2. Si `skills` no está vacío y `skillsRepo` está configurado → instala skills (comportamiento actual).
3. Si `agents` no está vacío y `agentsRepo` está configurado → instala agents (nuevo).
4. Reporte final: skills actualizados + agents actualizados + errores.

### `strand sync` (modificado)

1. Lee `config.json`.
2. Si `skills` no está vacío → sincroniza skills (comportamiento actual).
3. Si `agents` no está vacío → sincroniza agents (nuevo).
4. Reporte final: skills actualizados + agents actualizados + errores.

### `strand agents ls`

Espejo exacto de `strand ls` pero opera sobre `config.agents` y `agentsRepo`.

### `strand agents ls-remote`

Espejo exacto de `strand ls-remote` pero lista el directorio `agents/` del repo remoto en lugar de `skills/`.

### `strand agents validate`

Espejo exacto de `strand validate` pero escanea `.agents/agents/` en lugar de `.agents/skills/`.

---

## Init (modificado)

### Nuevo flujo interactivo

```text
# Preguntas existentes (sin cambios)
Enable Codex integration? [y/N]
GitLab project path for skills repository (e.g., namespace/project):
Branch or tag to use [main]:

# Nuevas preguntas
GitLab project path for agents repository (e.g., namespace/project) [optional]:
Branch or tag to use for agents [main]:
```

### Nuevas acciones

- Crea `.agents/agents/` (nuevo).
- Si `targets.opencode == true` (default): crea `.opencode/agents/` → symlink a `.agents/agents/`.
- Si `targets.codex == true`: crea `.codex/agents/` → symlink a `.agents/agents/`.
- Escribe `agentsRepo` en `config.json` (solo si el usuario proporcionó un project path).

---

## Install / Sync: Post-install Hooks para Agents

Al instalar o actualizar un agent:

1. Descarga el directorio completo `agents/<name>/` a `.agents/agents/<name>/`.
2. Actualiza `config.json` → `agents` array.
3. Asegura `.gitignore`:
   ```gitignore
   .agents/agents/<name>
   .opencode/agents/<name>
   .codex/agents/<name>
   ```
4. Si `targets.opencode == true`: crea/actualiza symlink `.opencode/agents/<name>` → `.agents/agents/<name>`.
5. Si `targets.codex == true`: crea/actualiza symlink `.codex/agents/<name>` → `.agents/agents/<name>`.

---

## Symlink Strategy

| CLI | Directorio esperado | Symlink origen | Symlink destino |
|-----|---------------------|----------------|-----------------|
| OpenCode | `.opencode/agents/` | `.opencode/agents/<name>` | `.agents/agents/<name>` |
| Codex | `.codex/agents/` | `.codex/agents/<name>` | `.agents/agents/<name>` |

La misma lógica de `codex.rs` se extiende o se generaliza para manejar ambos targets.

---

## Environment Variables

Se agregan variables espejo para agents:

| Variable | Uso |
|----------|-----|
| `strand_AGENTS_REPO` | Project path del repo de agents (override de config). |
| `strand_AGENTS_REPO_BRANCH` | Branch del repo de agents. |

Las existentes para skills (`strand_SKILLS_REPO`, `strand_SKILLS_REPO_BRANCH`, `strand_GITLAB_URL`) siguen funcionando igual.

---

## Implementation Plan

### 1. Model Layer

- Crear `src/models/agent.rs`:
  - `Agent` struct (espejo de `Skill`).
  - `AgentFrontmatter` struct (espejo de `SkillFrontmatter`).
  - `parse_agent_md()` (espejo de `parse_skill_md()`).

### 2. Config Layer

- Modificar `src/config.rs`:
  - Agregar `agents_repo: AgentsRepoConfig` (mismo shape que `SkillsRepoConfig`).
  - Agregar `agents: Vec<AgentEntry>` (mismo shape que `SkillEntry`).
  - Agregar `add_agent()` (espejo de `add_skill()`).

### 3. CLI Layer

- Modificar `src/cli.rs`:
  - Agregar `Commands::Agents { subcommand: AgentsCommands }`.
  - Crear `AgentsCommands` enum con: `Ls`, `LsRemote`, `Validate`.

### 4. Commands Layer

- Crear `src/commands/agents/`:
  - `mod.rs` — exporta submódulos.
  - `ls.rs` — espejo de `commands/ls.rs`.
  - `ls_remote.rs` — espejo de `commands/ls_remote.rs`.
  - `validate.rs` — espejo de `commands/validate.rs`.
- Modificar `src/commands/mod.rs` — exportar `pub mod agents`.
- Modificar `src/commands/init.rs`:
  - Preguntar por agents repo.
  - Crear `.agents/agents/`.
  - Crear symlinks `.opencode/agents/` y `.codex/agents/` según targets.
- Modificar `src/commands/install.rs`:
  - Después de iterar `config.skills`, iterar `config.agents`.
  - Extraer helpers compartidos si es necesario.
- Modificar `src/commands/sync.rs`:
  - Después de iterar `config.skills`, iterar `config.agents`.
- Modificar `src/commands/ls.rs` y `src/commands/ls_remote.rs`:
  - Sin cambios directos (siguen siendo skills-only por default).

### 5. Symlink / Post-install Layer

- Extender `src/codex.rs` o crear `src/symlinks.rs`:
  - Función genérica `create_symlink(target_dir: &str, link_dir: &str, name: &str)`.
  - Usada por skills (`.codex/skills/`) y agents (`.opencode/agents/`, `.codex/agents/`).

### 6. Gitignore Layer

- Modificar `src/gitignore.rs`:
  - Agregar `ensure_gitignore_entries_for_agent(name: &str)`.
  - O generalizar la función existente para recibir el tipo (skill/agent) y los paths.

### 7. Main Entrypoint

- Modificar `src/main.rs`:
  - Agregar match arm para `Commands::Agents { subcommand }`.
  - Dispatch a `commands::agents::ls::execute()`, etc.

### 8. Lib Export

- Modificar `src/lib.rs`:
  - Agregar `pub mod models::agent` (o `pub use models::agent`).
  - Agregar `pub mod commands::agents`.

---

## Backward Compatibility

- `config.json` sin `agentsRepo` ni `agents` es válido y funciona exactamente igual que hoy.
- `strand init` sin preguntar por agents repo: si el usuario presiona Enter en la pregunta de agents (dejándola vacía), no se escribe `agentsRepo` en el config.
- Todos los comandos de skills (`ls`, `ls-remote`, `install`, `sync`, `validate`) siguen operando **solo sobre skills**, salvo `install` y `sync` que ahora también procesan agents si están configurados.

---

## Test Strategy

- Unit tests para `parse_agent_md()` (mismo coverage que `parse_skill_md()`).
- Unit tests para `add_agent()` en `config.rs`.
- Unit tests para `install` y `sync` con config que contiene solo agents, solo skills, y ambos.
- Integration tests para `strand agents ls`, `strand agents ls-remote`, `strand agents validate`.

---

## Future Extensions

- `strand uninstall <name> --type skill|agent`
- Fuzzy search combinado skills+agents (`strand search`)
- Cross-artifact dependencies (un skill que depende de un agent)
