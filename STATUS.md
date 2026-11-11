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

Hecho: los scrapers de las tres cadenas andan y el sitio estático se genera completo (HTML con las ofertas, SEO, PWA instalable y offline, búsqueda y guardados). Probado en local con Chrome. Última corrida (10/11/2026): Carrefour 1.643 ofertas, Coto 4.766 (solo súper), DAR 280.

Publicado el 11/11/2026 en **https://ofertas-rosario.rindelloezequiel.workers.dev** (Cloudflare Workers). Falta: cargar los secrets de Cloudflare en el repo y probar el workflow de GitHub Actions.

**Próximo paso:** el mantenedor carga `CLOUDFLARE_API_TOKEN` (permiso "Workers Scripts · Edit") y `CLOUDFLARE_ACCOUNT_ID` en GitHub, y se corre el workflow a mano.

## Estado por componente

| Componente | Estado | Dónde |
| --- | --- | --- |
| Scraper Carrefour | ✅ Anda | `src/scrapers/carrefour.rs` |
| Scraper Coto | ✅ Anda | `src/scrapers/coto.rs` |
| Scraper DAR | ✅ Anda | `src/scrapers/dar.rs` |
| Detector (sanidad + % descuento) | ✅ Básico | `src/detector.rs` |
| Salida `data/` | ✅ Anda | `src/output.rs`, `data/offers/*.json`, `data/run.json` |
| Config de cadenas | ✅ | `config/stores.toml` |
| Tests | ✅ 15 tests con respuestas guardadas | `tests/fixtures/`, `#[cfg(test)]` en cada módulo |
| Control de caídas | ✅ Si una cadena trae menos de la mitad que ayer, se toma como error y se conserva el archivo anterior | `check_volume` en `src/main.rs` |
| Sitio estático (HTML, SEO) | ✅ Portada, una sección paginada por cadena, datos abiertos, 404, JSON-LD, sitemap, robots, llms.txt | `src/site.rs`, `templates/`, `config/site.toml` |
| PWA (manifest, service worker, aviso de instalación) | ✅ Chrome la ofrece para instalar; offline con precache | `web/manifest.webmanifest`, `web/sw.js`, `web/app.js` |
| Búsqueda y guardados | ✅ Búsqueda sin tildes sobre `datos/buscar.json`; guardados en `localStorage` | `web/app.js` |
| Íconos | 🟡 Provisorios (un "%" blanco sobre negro) | `web/icons/` |
| Deploy Cloudflare | ✅ Publicado (manual con `wrangler`) | `wrangler.jsonc` |
| GitHub Actions diario | 🟡 Escrito, sin probar (faltan los secrets) | `.github/workflows/daily.yml` |
| README para usuarios | 🟡 Versión mínima, falta el link a la web | `README.md` |
| Licencia | ✅ MIT | `LICENSE` |
| Historial de precios (baja real) | ⏳ Etapa 2 | |

## Cómo correrlo

Requisito: Rust estable ([rustup](https://rustup.rs)). En Windows conviene WSL: compilar en Windows nativo pide las herramientas de C++ de Visual Studio, y WSL es el mismo Linux que usa GitHub Actions.

```bash
cargo test                                     # tests con respuestas guardadas, sin red
cargo run --release -- scrape                  # todas las cadenas, escribe data/
cargo run --release -- scrape --only dar,coto  # solo algunas
cargo run --release -- build                   # genera el sitio en dist/ a partir de data/
python -m http.server 8787 --directory dist    # verlo en http://127.0.0.1:8787 (sin _headers)
npx wrangler@4 deploy                          # publicar dist/ en Cloudflare
```

Cerrá el servidor local antes de volver a hacer `build`: en Windows deja `dist/` tomado y el build falla con "Permission denied".

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
| 11/11/2026 | Coto: se excluyen las categorías Electro, Textil, Hogar y Aire Libre (`exclude_categories` en `config/stores.toml`) | El catálogo de Coto incluye de todo; sin filtro, lo más destacado eran velas y zapatillas. Bajó de 13.400 a 4.766 ofertas. |
| 11/11/2026 | Plantillas con `minijinja` embebidas en el binario; assets (`web/`) también embebidos | `dist/` se genera con un solo binario, sin archivos sueltos que copiar. |
| 11/11/2026 | Páginas de cadena de a 100 ofertas (`/coto/`, `/coto/pagina/2/`...), ordenadas por descuento | Páginas livianas en el celu y rastreables por buscadores. |
| 11/11/2026 | Búsqueda en el navegador sobre `datos/buscar.json` (filas compactas) | Sin backend. 1,4 MB sin comprimir; Cloudflare lo sirve comprimido. |
| 11/11/2026 | Service worker "network first" con caché de respaldo; la versión del caché cambia en cada build | Los precios cambian todos los días: primero la red, y si no hay señal, lo último guardado. |
| 11/11/2026 | Control de caídas en el binario: menos de la mitad de ofertas que el día anterior = error | Un rediseño suele verse como "pocas ofertas", no como un error. Mejor mostrar datos de ayer que una cadena vacía. |
| 11/11/2026 | Si alguna cadena falla, el workflow abre un issue en GitHub | Enterarse sin mirar los logs. |
| 11/11/2026 | URL pública: el subdominio `workers.dev` de la cuenta, por ahora | Gratis e inmediato. Un dominio propio queda para más adelante. |

## Próximos pasos

En orden. Cuando cierres una, marcala y poné el commit.

1. [x] **Sitio estático**: comando `build`, portada, una sección paginada por cadena, datos abiertos (`datos/<cadena>.json`, `datos/hoy.csv`), 404. HTML con las ofertas escritas, `title`, `description`, canonical, Open Graph, `lang="es-AR"` y JSON-LD. Sin estilos. (`c2bbbf9`)
2. [x] **SEO técnico**: `sitemap.xml`, `robots.txt` abierto a buscadores y bots de IA, `llms.txt`. (`c2bbbf9`)
3. [x] **PWA**: manifest, `sw.js` offline, botón "Instalar" y texto para iPhone/Android; se oculta si ya está instalada. (`c2bbbf9`)
4. [x] **JS del navegador**: búsqueda y guardados en `localStorage`. (`c2bbbf9`)
5. [x] **Deploy**: `npx wrangler@4 deploy` el 11/11/2026, Worker `ofertas-rosario`. URL en `config/site.toml`; todas las rutas responden y Cloudflare comprime (buscar.json: 1,4 MB → 248 KB). (commit pendiente)
6. [ ] **GitHub Actions**: cargar los secrets en el repo (los de otros repos no sirven: los secrets son por repo y no se pueden leer). `CLOUDFLARE_API_TOKEN` tiene que tener permiso "Workers Scripts · Edit" (plantilla "Edit Cloudflare Workers"). Después correrlo a mano (`workflow_dispatch`) y verificar que las tres cadenas respondan desde las IPs de Azure.
7. [x] **Control de caídas**: menos de la mitad de ofertas que ayer = error; el workflow abre un issue si alguna cadena falla. (`c2bbbf9`)
8. [ ] **README**: completar con el link a la web, capturas y cómo instalarla en el celu.
9. [ ] **Alta en buscadores** (la hace el dueño del dominio): Google Search Console y Bing Webmaster.
10. [ ] Testeo de punta a punta. Recién después: estilos.

## Lanzamiento (cuando el MVP esté terminado)

El repo es **privado** hasta terminar el MVP. Al lanzarlo:

- [ ] Hacer público el repo.
- [ ] Agregar `CONTRIBUTING.md` (en español: cómo correrlo, cómo sumar una cadena o corregir una sucursal, reglas de cortesía con los sitios).
- [ ] Crear la rama `development` para el trabajo diario y proteger `main` (solo entra por PR).
- [ ] Sacar el primer tag de release (`v0.1.0`).
- [ ] Poner la URL del repo en el User-Agent (`config/stores.toml`) y en el pie de la web.

## Pendientes y dudas abiertas

- [ ] Dominio propio (opcional, mejor para SEO). Si se cambia, actualizar `url` en `config/site.toml` y el link del README.
- [ ] Al proteger `main` en el lanzamiento, el workflow ya no va a poder pushear `data/` directo. Opciones: permitir el bot en la regla, o mover los datos a una rama `data`.
- [ ] Íconos definitivos (van con los estilos, al final).
- [ ] Filtros por categoría en la web: Carrefour se scrapea por categoría y Coto trae `allAncestors`, pero `Offer` todavía no guarda la categoría.
- [ ] Coto: guardar los descuentos por medio de pago (`product.dtoDescuentosMediosPago`) para la página de bancos.
- [ ] Coto: confirmar que `sDisp` 1004 = disponible y 1001 = no disponible.
- [ ] Carrefour: promos 2x1 (`Teasers` viene vacío en Intelligent Search).
- [ ] DAR: promos bancarias y `Detalle.asp` (sin revisar).
- [ ] DAR: no tiene precio anterior; su descuento real depende del historial (etapa 2).
- [ ] Revisar términos de uso de cada sitio antes del deploy público.
- [ ] Nombre del proyecto y dominio.
- [ ] Aclarar en el README y en la web que los datos salen de las webs públicas de cada cadena y pueden diferir del precio en góndola.

## Problemas conocidos

- El service worker precachea `datos/buscar.json` (1,4 MB sin comprimir) al instalarse. Si molesta en datos móviles, pasarlo a caché bajo demanda.
- Una corrida completa tarda ~3,5 minutos, la mayor parte en Carrefour (pausa de 1 s entre requests, 12 categorías).
- La ventana de Chrome maximizada no se deja achicar con la extensión; para ver ancho de celular se usó un iframe de 390 px.

## Commits

Del más nuevo al más viejo. Cada commit que cambie algo importante se agrega acá.

| Commit | Fecha | Qué |
| --- | --- | --- |
| `c2bbbf9` | 11/11/2026 | Sitio estático (portada, secciones por cadena, datos abiertos, SEO, `llms.txt`), PWA offline, búsqueda y guardados, filtro de categorías de Coto, control de caídas, `wrangler.jsonc` y workflow diario. |
| `c65fdc1` | 11/11/2026 | Primer commit: scrapers de Carrefour, Coto y DAR, detector, salida en `data/`, tests, docs, licencia y datos de la primera corrida. |
