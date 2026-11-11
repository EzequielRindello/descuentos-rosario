# STATUS: dónde está el proyecto

Leé esto primero si llegás sin contexto (persona o agente). Dice qué queremos, por dónde vamos, qué se decidió y qué falta.

**Regla:** este archivo se actualiza en cada cambio que importe. Cada tarea terminada cita el commit que la cerró (hash corto). Los commits se hacen a mano; el hash se carga después. Si algo de acá no coincide con el código o con `git log`, manda el código: corregí este archivo.

Última actualización: 11/11/2026.

## Qué es

**Ofertas Rosario**: una web pública que todos los días junta las ofertas de los supermercados de Rosario (Argentina) y muestra las que valen la pena. Es open source y no tiene login. Se puede instalar en el celu como app (PWA) y funciona sin señal adentro del súper. Los datos del día también se publican como JSON/CSV.

Documentos:

| Archivo | Para qué |
| --- | --- |
| `STATUS.md` | Este archivo: estado, decisiones, tareas. |
| `docs/context.md` | Referencia técnica: stack, reglas de cortesía, cómo se lee cada fuente (endpoints, parámetros, campos), formato de los datos. |
| `docs/brief.md` | El porqué: investigación inicial, sucursales, criterios de descuento, roadmap y riesgos. |
| `AGENTS.md` | Instrucciones para agentes de código (`CLAUDE.md` apunta acá). |
| `README.md` | Para usuarios: qué es y dónde verlo. |

## Por dónde vamos

Hecho: los scrapers de las tres cadenas del MVP andan y escriben el JSON del día. Primera corrida real (10/11/2026): Carrefour 1.643 ofertas, Coto 13.400, DAR 280, en ~3,5 minutos.

Falta: generar el sitio (HTML + PWA), el deploy a Cloudflare, la corrida diaria en GitHub Actions y el README.

**Próximo paso:** generar el sitio estático a partir de `data/` (tarea 1 de "Próximos pasos").

## Estado por componente

| Componente | Estado | Dónde |
| --- | --- | --- |
| Scraper Carrefour | ✅ Anda | `src/scrapers/carrefour.rs` |
| Scraper Coto | ✅ Anda | `src/scrapers/coto.rs` |
| Scraper DAR | ✅ Anda | `src/scrapers/dar.rs` |
| Detector (sanidad + % descuento) | ✅ Básico | `src/detector.rs` |
| Salida `data/` | ✅ Anda | `src/output.rs`, `data/offers/*.json`, `data/run.json` |
| Config de cadenas | ✅ | `config/stores.toml` |
| Tests | ✅ 11 tests con respuestas guardadas | `tests/fixtures/`, `#[cfg(test)]` en cada módulo |
| Sitio estático (HTML, SEO) | ⏳ No empezado | |
| PWA (manifest, service worker, aviso de instalación) | ⏳ No empezado | |
| Deploy Cloudflare | ⏳ No empezado | |
| GitHub Actions diario | ⏳ No empezado | |
| README para usuarios | 🟡 Versión mínima, falta el link a la web | `README.md` |
| Licencia | ✅ MIT | `LICENSE` |
| Historial de precios (baja real) | ⏳ Etapa 2 | |

## Cómo correrlo

Requisito: Rust estable ([rustup](https://rustup.rs)). En Windows conviene WSL: compilar en Windows nativo pide las herramientas de C++ de Visual Studio, y WSL es el mismo Linux que usa GitHub Actions.

```bash
cargo test                                   # tests con respuestas guardadas, sin red
cargo run --release -- scrape                # todas las cadenas, escribe data/
cargo run --release -- scrape --only dar,coto  # solo algunas
```

Si el repo está en un disco de Windows y compilás desde WSL, apuntá `CARGO_TARGET_DIR` a una carpeta del home de WSL: compilar sobre `/mnt/...` es lento.

## Decisiones

| Fecha | Decisión | Por qué |
| --- | --- | --- |
| 10/11/2026 | Web estática, sin login, sin backend | Deploy simple, costo cero, nada que mantener. |
| 10/11/2026 | PWA instalable (manifest + service worker) | "Descargar la app" sin tiendas. Botón en Android, instrucciones en iPhone. |
| 10/11/2026 | Sin notificaciones en el MVP. Telegram y WhatsApp descartados | Telegram suma un canal externo que no se quiere mantener. WhatsApp no tiene API gratis para canales y la oficial pide guardar teléfonos. Más adelante: push de la propia PWA con un Worker. |
| 10/11/2026 | Nada de estilos, fuentes ni colores hasta que la app ande y esté testeada | Prioridad: que funcione. |
| 10/11/2026 | Rust para scrapers y generación del sitio; JS plano en el navegador | Eficiencia, un solo binario, JSON tipado que falla fuerte si una cadena cambia el formato. SEPA (4 GB) se procesa en streaming. |
| 10/11/2026 | HTML generado de antemano, no SPA | SEO y que las IAs puedan leer y recomendar el sitio. |
| 10/11/2026 | Cloudflare Workers con static assets + `wrangler deploy`, sin conectar git | Cloudflare recomienda Workers sobre Pages, y el Worker sirve después para push. |
| 10/11/2026 | GitHub Actions corre el scrape diario y hace el deploy | Algo tiene que correr todos los días sin depender de una PC prendida. |
| 10/11/2026 | Todo en español menos el código. README pensado para usuarios | El público es de Rosario. |
| 10/11/2026 | Desarrollo en Linux (WSL en Windows) | Es el mismo entorno que GitHub Actions y no pide las herramientas de C++ de Visual Studio. |
| 10/11/2026 | User-Agent identificable, respetar `robots.txt`, nunca hacerse pasar por un navegador ni sacar keys de las páginas | Es un proyecto público: tiene que poder mostrar cómo lee cada sitio sin esconder nada. |
| 11/11/2026 | La Gallega fuera del MVP | Su `robots.txt` (`Disallow: /*?`) bloquea todas las páginas de ofertas. |
| 11/11/2026 | Coto por el endpoint viejo Endeca, no por la API nueva | La API nueva pide una key de Constructor.io. Endeca no, y trae EAN y disponibilidad por tienda. |
| 11/11/2026 | Coto: precio online (lista 200) filtrado por disponibilidad en Alto Rosario (165) | Endeca no expone la lista 165. El brief vio que coinciden. Se valida con SEPA más adelante. |
| 11/11/2026 | DAR con OpenSSL compilado dentro del binario (`native-tls-vendored`), solo para DAR | Su servidor solo acepta un cifrado CBC viejo que rustls no soporta. Vendored evita instalar paquetes del sistema. |
| 11/11/2026 | Precios en centavos enteros; un archivo por cadena, una oferta por línea, ordenado | Sin errores de redondeo y diffs de git chicos. |
| 11/11/2026 | Si una cadena falla, se conserva su archivo del día anterior | La web nunca queda vacía; el error queda en `data/run.json`. |
| 11/11/2026 | Estructura del repo: en la raíz solo README, LICENSE, STATUS y AGENTS; documentación en `docs/` | Lo primero que ve alguien que entra al repo es lo que le sirve. |
| 11/11/2026 | `AGENTS.md` como instrucciones para agentes, con `CLAUDE.md` apuntando ahí | `AGENTS.md` lo leen varias herramientas (Codex, Cursor, Copilot, Claude). |
| 11/11/2026 | Fixtures de tests recortadas a los campos que se usan (~15 KB en total) | Repo liviano y sin redistribuir respuestas completas de las APIs de las cadenas. |
| 11/11/2026 | `data/` se commitea en `main` | Son los datos abiertos y el historial. Si el repo pesa demasiado, se mueve a una rama `data` sin perder historial. |
| 11/11/2026 | Licencia MIT, a nombre de "Ofertas Rosario contributors" | Open source sin datos personales en el archivo. |
| 11/11/2026 | `.gitattributes` con `eol=lf` | Los datos los escribe Linux (Actions); sin esto los diffs pueden marcar archivos enteros por fin de línea. |

## Próximos pasos

En orden. Cuando cierres una, marcala y poné el commit.

1. [ ] **Sitio estático**: comando `build` que lee `data/` y genera `dist/` con una portada (mejores ofertas del día), una página por cadena, `datos/hoy.json` y `datos/hoy.csv`. HTML con las ofertas escritas, `title`, `description`, canonical, Open Graph, `lang="es-AR"` y JSON-LD (`ItemList` + `Offer`). Sin estilos.
2. [ ] **SEO técnico**: `sitemap.xml`, `robots.txt` (permitir GPTBot, ClaudeBot, PerplexityBot, Google-Extended), `llms.txt`.
3. [ ] **PWA**: `manifest.webmanifest`, `sw.js` que guarda las ofertas del día para usar offline, botón "Instalar" (`beforeinstallprompt`) e instrucciones para iPhone. El aviso de instalación se oculta si la app ya corre instalada (`matchMedia('(display-mode: standalone)')`).
4. [ ] **JS del navegador**: búsqueda y filtros sobre el JSON, favoritos en `localStorage`.
5. [ ] **Deploy**: `wrangler.jsonc` con static assets y primer `wrangler deploy` manual (con una cuenta de Cloudflare logueada en `wrangler`).
6. [ ] **GitHub Actions** (`.github/workflows/daily.yml`): cron diario, `cargo run --release -- scrape`, `build`, `wrangler deploy` con `CLOUDFLARE_API_TOKEN`, commit de `data/`. Verificar que las tres cadenas respondan desde las IPs de Azure.
7. [ ] **Control de caídas**: que la corrida falle o abra un issue si una cadena trae menos del 50% de lo habitual.
8. [ ] **README**: completar con el link a la web, capturas y cómo instalarla en el celu.
9. [ ] **Alta en buscadores** (la hace el dueño del dominio): Google Search Console y Bing Webmaster.
10. [ ] Testeo de punta a punta. Recién después: estilos.

## Pendientes y dudas abiertas

- [ ] User-Agent: poner la URL del repo cuando exista (`config/stores.toml`).
- [ ] Coto: confirmar que la URL de producto que arma el scraper abre el producto en el sitio nuevo (SPA).
- [ ] Coto: guardar los descuentos por medio de pago (`product.dtoDescuentosMediosPago`) para la página de bancos.
- [ ] Coto: confirmar que `sDisp` 1004 = disponible y 1001 = no disponible.
- [ ] Carrefour: promos 2x1 (`Teasers` viene vacío en Intelligent Search).
- [ ] DAR: promos bancarias y `Detalle.asp` (sin revisar).
- [ ] DAR: no tiene precio anterior; su descuento real depende del historial (etapa 2).
- [ ] Revisar términos de uso de cada sitio antes del deploy público.
- [ ] Nombre del proyecto y dominio.
- [ ] Aclarar en el README y en la web que los datos salen de las webs públicas de cada cadena y pueden diferir del precio en góndola.

## Problemas conocidos

- `data/offers/coto.json` pesa ~6 MB. La web no tiene que cargarlo entero en el celu: partir por categoría o armar un `top.json` liviano (tarea 1).
- Una corrida completa tarda ~3,5 minutos, la mayor parte en Carrefour (pausa de 1 s entre requests, 12 categorías).

## Commits

Del más nuevo al más viejo. Cada commit que cambie algo importante se agrega acá.

| Commit | Fecha | Qué |
| --- | --- | --- |
| (sin commits todavía) | 11/11/2026 | Repo inicializado con `git init`. Scrapers, docs, licencia y datos de la primera corrida esperando el primer commit. |
