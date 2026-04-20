# Migración MySQL → PostgreSQL

El script `tools/migrate_mysql_to_pg.py` migra los datos de la instalación PHP original (MySQL) al nuevo backend Rust (PostgreSQL).

---

## Requisitos

```bash
pip install mysql-connector-python psycopg2-binary
```

---

## Uso

```bash
python tools/migrate_mysql_to_pg.py \
  --mysql-host 127.0.0.1 \
  --mysql-db Jungle_db \
  --mysql-user root \
  --mysql-password <password> \
  --pg-host 127.0.0.1 \
  --pg-db Jungle \
  --pg-user Jungle \
  --pg-password <password>
```

---

## Qué Migra

El script migra **60+ tablas** con las siguientes transformaciones:

### Conversiones de Tipos

| MySQL | PostgreSQL | Notas |
|-------|-----------|-------|
| `TINYINT(1)` | `BOOLEAN` | 0 → false, 1 → true |
| `INT UNSIGNED` | `BIGINT` | Evita overflow |
| `DATETIME` | `TIMESTAMPTZ` | Añade timezone UTC |
| `TEXT` con JSON | `JSONB` | Parsea y valida JSON |
| `ENUM(...)` | `VARCHAR(50)` | Mantiene compatibilidad |
| `MEDIUMTEXT` | `TEXT` | Sin límite en PG |

### Consolidaciones de Tablas

Algunas tablas PHP se consolidan en el nuevo esquema:

| Tablas MySQL | Tabla PostgreSQL | Descripción |
|-------------|-----------------|-------------|
| `Oc_users`, `Oc_user_data` | `users` | Perfil unificado |
| `Oc_likes`, `Oc_post_reactions` | `reactions` | Reacciones unificadas |
| `Oc_messages`, `Oc_chat` | `messages` | Mensajes unificados |

### Resets de Secuencias

Tras la migración, el script resetea todas las secuencias PostgreSQL al valor máximo de cada tabla para evitar conflictos de IDs:

```sql
SELECT setval('users_id_seq', (SELECT MAX(id) FROM users));
```

### Verificación

Al finalizar, el script verifica que el conteo de filas en PostgreSQL coincide con MySQL para cada tabla migrada.

---

## Proceso de Migración Recomendado

1. **Preparar el entorno**:
   ```bash
   docker compose up -d postgres
   cargo run -p auth-service  # ejecuta migraciones SQL
   ```

2. **Ejecutar migración**:
   ```bash
   python tools/migrate_mysql_to_pg.py ...
   ```

3. **Verificar datos**:
   ```bash
   psql -U Jungle -d Jungle -c "SELECT COUNT(*) FROM users;"
   ```

4. **Arrancar todos los servicios**:
   ```bash
   docker compose up -d
   ```

5. **Verificar salud**:
   ```bash
   curl http://localhost:8080/health
   ```

---

## Notas Importantes

- La migración es **no destructiva** — no modifica la base de datos MySQL original.
- Se recomienda hacer la migración con el sitio en **modo mantenimiento** para evitar datos inconsistentes.
- Los passwords se migran tal cual (ya están hasheados en MySQL). El nuevo sistema soporta tanto bcrypt (legacy) como Argon2id (nuevo).
- Los archivos de media (imágenes, videos) deben migrarse por separado al nuevo proveedor de almacenamiento (MinIO/S3).
