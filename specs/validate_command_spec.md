# strand validate — Skill Repository Validator

## Overview

Nuevo comando `strand validate` que analiza un repositorio de skills compartido y verifica que todos los skills cumplan con el spec de formato JSON y estructura de archivos.

## Contexto

El repositorio de skills tiene esta estructura:

```
skills-repo/
  skills/
    react-native-review/
      skill.json
      skill.md
      assets/
        logo.png
    gitlab-ci-review/
      skill.json
      skill.md
```

Cada `skill.json` debe contener:

```json
{
  "name": "react-native-review",
  "description": "Reviews React Native code and architecture",
  "version": "1.2.0",
  "entrypoint": "skill.md"
}
```

## Comando

```bash
# En el directorio del repo de skills
strand validate
```

## Funcionalidad

1. **Descubrimiento**: Encontrar todos los subdirectorios en `skills/`
2. **Validación de estructura**: Verificar que cada skill tenga:
   - Archivo `skill.json` presente y parseable
   - Campo `name` presente y no vacío
   - Campo `description` presente y no vacío
   - Campo `version` presente y válido según semver
   - Campo `entrypoint` presente y no vacío
   - Archivo entrypoint exista en el directorio del skill
   - Si hay directorio `assets/`, que los archivos referenciados existan
3. **Reporte**: Mostrar tabla con resultados:
   - Skill name
   - Estado (✓ válido / ✗ inválido)
   - Errores encontrados
4. **Auto-fix interactivo**: Si hay errores, preguntar:
   ```
   Found 3 skills with issues. Fix automatically? [y/N]
   ```
   - Si sí: intentar arreglar problemas simples (faltan campos, version malformada, etc.)
   - Si no: mostrar reporte y salir

## Validaciones específicas

### Errores críticos (no auto-fixable)
- skill.json no existe
- skill.json no es JSON válido
- Directorio del skill está vacío

### Errores auto-fixable
- Falta campo `name` → usar nombre del directorio
- Falta campo `description` → usar "No description provided"
- Falta campo `version` → usar "0.1.0"
- Versión no es semver válido → normalizar o usar "0.1.0"
- Falta campo `entrypoint` → usar "skill.md"
- Entrypoint no existe → crear archivo vacío con advertencia

## Salida

```
Validating skills repository...

Skill                  Status   Issues
react-native-review    ✓        None
gitlab-ci-review       ✗        Missing 'version' field
release-validator      ✗        Invalid semver in version: "1.2"

Summary: 1 valid, 2 invalid

Fix 2 auto-fixable issues? [y/N]:
```

## Dependencias

- Módulo de semver existente (ya en Cargo.toml)
- serde_json para parseo
- dialoguer para prompts interactivos (ya disponible)

## Tests

- Test con repo válido
- Test con skill.json faltante
- Test con JSON inválido
- Test con campos faltantes
- Test con versión semver inválida
- Test de auto-fix

## Out of scope

- Validación de contenido de skill.md
- Validación de assets (solo verificar existencia)
- Soporte para subir cambios al repositorio
