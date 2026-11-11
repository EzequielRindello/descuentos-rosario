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

Publicado el 11/11/2026 en **https://ofertas-rosario.rindelloezequiel.workers.dev** (Cloudflare Workers). Los secrets de Cloudflare están cargados en GitHub y el workflow diario está activo.

Primera corrida en GitHub (a mano, 11/11/2026, run `43022603196`): las tres cadenas respondieron desde Actions (no hay bloqueo por IP) y el deploy anduvo. Falló en dos cosas: el control de caídas frenó a Coto (que había terminado sus promos de verdad) y el push de `data/` fue rechazado porque se pusheó otro commit mientras corría. Las dos están en Decisiones y Problemas conocidos. Los datos se verificaron a mano contra cada sitio (Carrefour 15/15, Coto 25/25 más 12 ofertas terminadas, DAR 7/7).

Segunda corrida manual (11/11/2026, run `43030708828`, con `1e5ae5f`): todo OK. Carrefour 1.616, Coto 1.114, DAR 287; deploy y commit de `data/` (`c287d33`) funcionaron. Issue #1 cerrado.

**Próximo paso:** revisar la primera corrida por cron, el **12/11/2026 a las 07:00** (ver "Cómo revisar la corrida diaria" más abajo). Es lo único del workflow que falta ver: que el `schedule` dispare solo.

## Cómo revisar la corrida diaria

El workflow (`.github/workflows/daily.yml`) corre todos los días a las 07:00 de Argentina (GitHub puede demorarlo 5 a 30 minutos): tests, scrape, build, `wrangler deploy`, commit de `data/` en `main` ("data: ofertas del DD/MM/AAAA") e issue en GitHub si alguna cadena falló.

```bash
gh run list --workflow daily.yml --limit 5     # ¿corrió? ¿terminó bien?
gh run view <id> --log-failed                  # si falló, qué paso falló
gh issue list                                  # issues "Falló el scrape del ..." que abre el workflow
git pull                                       # traer el commit de data/ que hizo el bot
cat data/run.json                              # ofertas y error por cadena de la última corrida
```

También se ve en la web: el pie de cada página dice "Ofertas actualizadas el ...".

Si una cadena falla solo desde GitHub (anda en local y no en Actions), lo más probable es un bloqueo por IP. No disfrazar el User-Agent ni saltear protecciones (ver reglas en `AGENTS.md`): anotarlo acá y discutir alternativas con el mantenedor.

Para correrlo a mano sin esperar al cron: `gh workflow run daily.yml` (publica y commitea de verdad; pedir OK al mantenedor antes).

## Estado por componente

| Componente | Estado | Dónde |
| --- | --- | --- |
| Scraper Carrefour | ✅ Anda | `src/scrapers/carrefour.rs` |
| Scraper Coto | ✅ Anda | `src/scrapers/coto.rs` |
| Scraper DAR | ✅ Anda | `src/scrapers/dar.rs` |
| Detector (sanidad + % descuento) | ✅ Básico | `src/detector.rs` |
| Salida `data/` | ✅ Anda | `src/output.rs`, `data/offers/*.json`, `data/run.json` |
| Config de cadenas | ✅ | `config/stores.toml` |
| Tests | ✅ 18 tests con respuestas guardadas | `tests/fixtures/`, `#[cfg(test)]` en cada módulo |
| Control de caídas | ✅ Si una cadena trae 0 ofertas (y ayer tenía), se toma como error y se conserva el archivo anterior. Las bajas grandes solo quedan en el log | `check_volume` en `src/main.rs` |
| Sitio estático (HTML, SEO) | ✅ Portada, una sección paginada por cadena, datos abiertos, 404, JSON-LD, sitemap, robots, llms.txt | `src/site.rs`, `templates/`, `config/site.toml` |
| PWA (manifest, service worker, aviso de instalación) | ✅ Chrome la ofrece para instalar; offline con precache | `web/manifest.webmanifest`, `web/sw.js`, `web/app.js` |
| Búsqueda y guardados | ✅ Búsqueda sin tildes sobre `datos/buscar.json`; guardados en `localStorage` | `web/app.js` |
| Íconos | 🟡 Provisorios (flecha de cursor blanca sobre negro, más `favicon.svg`) | `web/icons/` |
| Páginas legales | ✅ Preguntas frecuentes, términos y privacidad. Falta el canal de contacto y la revisión legal | `templates/preguntas.html`, `terminos.html`, `privacidad.html` |
| Deploy Cloudflare | ✅ Publicado (manual con `wrangler`) | `wrangler.jsonc` |
| GitHub Actions diario | 🟡 Corrida manual completa OK (11/11); falta ver que el cron dispare solo (12/11) | `.github/workflows/daily.yml` |
| README para usuarios | ✅ Link, cómo instalarla y datos abiertos. Las capturas van después de los estilos | `README.md` |
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
npx wrangler@4.155.0 deploy                     # publicar dist/ en Cloudflare (requiere `npx wrangler@4.155.0 login`)
```

El bot de GitHub Actions commitea `data/` en `main` todos los días: hacé `git pull` antes de trabajar.

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
| 11/11/2026 | Control de caídas en el binario: menos de la mitad de ofertas que el día anterior = error (reemplazada el mismo día: ahora solo 0 ofertas es error, ver más abajo) | Un rediseño suele verse como "pocas ofertas", no como un error. Mejor mostrar datos de ayer que una cadena vacía. |
| 11/11/2026 | Si alguna cadena falla, el workflow abre un issue en GitHub | Enterarse sin mirar los logs. |
| 11/11/2026 | URL pública: el subdominio `workers.dev` de la cuenta, por ahora | Gratis e inmediato. Un dominio propio queda para más adelante. |
| 11/11/2026 | Control de caídas: solo es error si una cadena trae 0 ofertas; antes lo era traer menos de la mitad que ayer | Las ofertas cambian mucho de un día a otro: el 11/11 Coto terminó sus promos y pasó de 4.766 a 1.114 ofertas reales (verificado a mano), y la regla vieja dejó en la web ofertas que ya no existían. |
| 11/11/2026 | Coto: si `precioLista` no es mayor que el precio de oferta, el precio regular sale de `textoPrecioRegular` ("Precio Contado: $X") | En "Otras Ofertas" (verdulería, pescadería, carnicería) `precioLista` es igual al precio de oferta y el regular solo está en ese texto, que es lo que la web de Coto muestra como "Precio regular". Suma 175 ofertas con descuento (11/11/2026). |
| 11/11/2026 | Si una cadena falla, el sitio la muestra vacía con un aviso ("Hoy no pudimos leer las ofertas de ..."), también en búsqueda y datos abiertos. `data/` igual conserva su último archivo bueno | La web muestra las ofertas del día, sean pocas o muchas; las de ayer pueden ya no valer. `data/` guarda el archivo para comparar y como historial. Lo decide `run.json` (`ok: false`). |
| 11/11/2026 | Si fallan todas las cadenas, el workflow igual genera y publica el sitio (y commitea `run.json`); la corrida queda en rojo | Antes cortaba en el scrape y la web se quedaba entera con las ofertas de ayer. |
| 11/11/2026 | `/datos/*` con `max-age=0, must-revalidate` en vez de una hora de caché | Con una hora, la búsqueda podía mostrar ofertas de ayer después de la actualización. Con ETag, revalidar cuesta un 304. |
| 11/11/2026 | Páginas de preguntas frecuentes, términos y privacidad, enlazadas desde el pie | Dejar por escrito que el sitio es informativo, sin relación con las cadenas, sin garantías, y cómo se leen los datos. No reemplaza una revisión legal (tarea 12). |
| 11/11/2026 | El contacto es un campo opcional de `config/site.toml` que no se muestra si está vacío | No publicar datos personales; el canal se define aparte (tarea 11). |
| 11/11/2026 | CSP estricta (`'self'` para todo, sin inline) | El sitio no carga nada de terceros. Si un día se cuela HTML o JS ajeno, el navegador no lo ejecuta. El JSON-LD inline no es JS ejecutable y no lo frena. |
| 11/11/2026 | Íconos generados con un script de Python sin dependencias (sin Pillow) | No descargar paquetes para algo provisorio. |
| 11/11/2026 | Contacto: el mail personal del mantenedor, por decisión suya (excepción a "sin datos personales", como el subdominio) | Las cadenas necesitan un canal para pedir correcciones o retiro; el mantenedor eligió no crear uno aparte. Puede recibir spam por estar publicado. |
| 11/11/2026 | Guardados con nombre (`[{ key, name }]` en `localStorage`); los que hoy no están en oferta se listan con su nombre y un botón "Descartar". El botón dice "Guardar" o "Descartar" | Antes solo decía "1 guardada ya no está en oferta" y no había forma de sacarla. Si la oferta vuelve, reaparece sola. Los guardados del formato viejo se leen igual y toman el nombre cuando aparecen en el índice. |
| 11/11/2026 | SEPA (Precios Claros) descartado | Decisión del mantenedor. Con eso quedan afuera Jumbo y Todo Chen (que no tiene web, solo Instagram) y la validación cruzada de precios. |
| 11/11/2026 | Actions del workflow fijadas por SHA, `wrangler` a versión exacta (de al menos dos semanas) y Dependabot | Una versión nueva comprometida correría con permiso de escribir en el repo y con el token de Cloudflare. Con SHA, lo que corre no cambia sin un PR. |
| 11/11/2026 | DAR figura solo como "DAR", sin sucursal ni dirección | La dirección que había era un dato personal del mantenedor. |
| 11/11/2026 | Capturas del README recién después de los estilos | Sin estilos no muestran nada útil. |
| 11/11/2026 | Carrefour: el precio es el del Hiper Rosario Fisherton (seller `carrefourar0018`, punto de retiro `carrefourar0018_0018PR`) y cada cadena tiene una aclaración (`hint` en `config/stores.toml`) de cómo ver el mismo precio en su web | En la web de Carrefour el código postal de Rosario (envío a domicilio) usa otra región con vendedores de otras ciudades: en una muestra, 4 de 20 precios eran distintos. El link no puede fijar la región (se guarda en una cookie de Carrefour), así que se explica: elegir retiro en Hiper Rosario Fisherton. |
| 11/11/2026 | Fotos de Carrefour y Coto desde sus servidores (sin copiarlas), DAR sin fotos | Decisión del mantenedor. No se copian las fotos (son de las cadenas y marcas); se cargan desde la cadena con `loading="lazy"` y `referrerpolicy="no-referrer"`. DAR sirve las fotos desde su propio servidor chico. El servidor de imágenes de Coto tiene `robots.txt` que pide a los bots no entrar: el scraper nunca baja imágenes, las carga el navegador de cada persona. Costo: las cadenas ven la IP de quien mira (dicho en Privacidad) y sin señal no hay fotos. |
| 11/11/2026 | `discount_pct` truncado en vez de redondeado | Nunca mostrar más descuento que el real; con redondeo, un 35,5% salía "-36%" al lado del "35%Dto" de Coto. Se ve desde la próxima corrida (se calcula al scrapear). |
| 11/11/2026 | Botón "¡Instalá la app!" en el header en vez de la sección "Instalala en el celu" | Pedido del mantenedor. Abre el diálogo de Chrome si está disponible; si no, muestra cómo hacerlo. Desaparece con la app instalada. |
| 11/11/2026 | `buscar.json` fuera del precache; la app instalada lo baja en segundo plano | Eran 108 KB de los ~150 KB de la primera visita, aunque la persona no buscara nada. Quien usa el sitio sin señal en el súper es quien lo instala. |
| 11/11/2026 | Filtros en la búsqueda de la portada (sobre `buscar.json`), no dentro de las páginas de cadena | Las páginas de cadena son HTML fijo de a 100 para buscadores; filtrar ahí solo vería esas 100. La búsqueda tiene todas las ofertas. Las páginas de cadena enlazan a la búsqueda con la cadena elegida. |
| 11/11/2026 | El formulario de búsqueda ya no usa `onsubmit` inline; el envío lo maneja `app.js` | La CSP bloquea los handlers inline: con Enter se recargaba la página. |
| 11/11/2026 | Aviso de fecha en la página en vez de cambiar la estrategia de caché | El caché sirve sin señal en el súper y no hay que perderlo. Lo que faltaba era decir de qué día son las ofertas cuando no son de hoy. "Todavía no están las de hoy" también cubre el rato antes de la corrida y una corrida fallida. |
| 11/11/2026 | El workflow no hace `pull --rebase` antes de pushear `data/` | Si se pushea algo mientras corre, que falle y quede registrado; se aprende de eso. Los datos de ese día igual quedan publicados en la web. |
| 11/11/2026 | El subdominio `workers.dev` lleva el nombre del mantenedor y se deja así por ahora (excepción a "sin datos personales") | Cambiarlo no aporta hasta tener dominio propio; se reemplaza junto con el dominio. |

## Próximos pasos

En orden. Cuando cierres una, marcala y poné el commit.

1. [x] **Sitio estático**: comando `build`, portada, una sección paginada por cadena, datos abiertos (`datos/<cadena>.json`, `datos/hoy.csv`), 404. HTML con las ofertas escritas, `title`, `description`, canonical, Open Graph, `lang="es-AR"` y JSON-LD. Sin estilos. (`c2bbbf9`)
2. [x] **SEO técnico**: `sitemap.xml`, `robots.txt` abierto a buscadores y bots de IA, `llms.txt`. (`c2bbbf9`)
3. [x] **PWA**: manifest, `sw.js` offline, botón "Instalar" y texto para iPhone/Android; se oculta si ya está instalada. (`c2bbbf9`)
4. [x] **JS del navegador**: búsqueda y guardados en `localStorage`. (`c2bbbf9`)
5. [x] **Deploy**: `npx wrangler@4 deploy` el 11/11/2026, Worker `ofertas-rosario`. URL en `config/site.toml`; todas las rutas responden y Cloudflare comprime (buscar.json: 1,4 MB → 248 KB). (`de3ec79`)
6. [ ] **GitHub Actions**: secrets `CLOUDFLARE_API_TOKEN` (token de cuenta con permiso Account · Workers Scripts · Edit) y `CLOUDFLARE_ACCOUNT_ID` cargados el 11/11/2026. Corrida manual del 11/11 completa: las tres cadenas responden desde GitHub, deploy y commit de `data/` andan (`c287d33`). Falta confirmar que el cron dispare solo (12/11/2026, 07:00).
7. [x] **Control de caídas**: 0 ofertas = error (al principio era "menos de la mitad que ayer"); el workflow abre un issue si alguna cadena falla. (`c2bbbf9`, ajustado después)
8. [ ] **README**: cómo instalarla en el celu y datos abiertos, hecho (`085a52f`). Faltan las capturas, que van después de los estilos.
9. [ ] **Alta en buscadores** (la hace el dueño del dominio): Google Search Console (propiedad "Prefijo de URL", verificación por archivo HTML en `web/`) y Bing Webmaster (importar desde Google). **Esperar** a la revisión de términos de las cadenas y al dominio propio.
10. [x] **Páginas legales e informativas**: `/preguntas-frecuentes/`, `/terminos/` (aviso legal: sin relación con las cadenas, precios informativos, sin garantías, cómo se leen los datos, pedidos de retiro, ley argentina) y `/privacidad/` (sin cookies ni analítica; verificado que Cloudflare no pone cookies). Links en el pie y en el sitemap. 404 verificado en producción. (`f6dc91a`, `71ce648`)
11. [x] **Canal de contacto**: un mail en `contact` de `config/site.toml`, con link `mailto:` en preguntas, términos y privacidad. Sirve para los pedidos de corrección o retiro de las cadenas. (`f6dc91a`, `71ce648`)
12. [ ] **Revisión legal**: los textos de términos y privacidad los escribió un agente, no un abogado. Que los revise alguien con matrícula antes del lanzamiento, junto con los términos de uso de cada cadena.
13. [x] **Favicon**: flecha de cursor blanca sobre negro (`web/icons/favicon.svg` + PNG 192/512/maskable). Provisorio hasta los estilos. (`f6dc91a`, `71ce648`)
14. [x] **Seguridad, arreglos rápidos**: headers en `web/_headers` (CSP que solo permite el propio sitio, `frame-ancestors 'none'`, Permissions-Policy, COOP, HSTS) y links a las cadenas solo si son `https://`. Probado en local con un servidor que aplica `_headers`: sin errores de CSP. Falta verlo en producción. (`f6dc91a`, `71ce648`)
15. [x] **Seguridad del workflow**: actions fijadas por SHA (`actions/checkout` v7.0.1, que ya corre en Node 24; `dtolnay/rust-toolchain` stable; `Swatinem/rust-cache` v2.9.3), `wrangler@4.155.0` exacto y Dependabot semanal para Cargo y Actions (`.github/dependabot.yml`). `wrangler` se actualiza a mano: Dependabot no ve `npx`. Falta verlo en una corrida real. (`957df38`)
16. [x] **Datos viejos en caché**: cada página lleva la fecha de sus datos (`body[data-updated]`) y `app.js` la compara con `/datos/actualizado.json`, que el service worker nunca cachea. Avisa "Hay ofertas más nuevas (Recargar)", "Sin conexión: estas ofertas son del DD/MM", o "Todavía no están las ofertas de hoy". Si todo es de hoy, nada. Vuelve a chequear al reconectarse o al volver a la app. Probados los cuatro casos en Chrome. (`0c125e0`)
17. [x] **Filtros y orden (UX)**: la búsqueda de la portada pasó a "Buscar y filtrar": producto, cadena (casillas), descuento mínimo (10 a 50%), tipo de promo (precio rebajado, 2x1 y 3x2, 2da unidad, llevando varias) y orden (descuento, precio, nombre). Se aplican al cambiar, sin texto también; resultados de a 50 con "Mostrar más"; los filtros quedan en la URL (`/?cadena=coto&min=40`). Cada página de cadena tiene "Filtrar las ofertas de ...". Probado en Chrome, también a 390 px. Por categoría falta guardar la categoría en `Offer` (ver Pendientes). (commit pendiente)
18. [ ] **Performance**: medido en producción (comprimido): portada 7,5 KB, página de cadena ~11 KB, `app.js` 2,6 KB, `buscar.json` 108 KB (583 KB sin comprimir). Hecho: `buscar.json` ya no se precachea en la primera visita (era ~70% de lo que bajaba); se guarda al buscar por primera vez, y la app instalada lo baja en segundo plano para buscar sin señal. Probado en Chrome: primera visita sin el índice, y búsqueda sin servidor después de haber buscado una vez (commit pendiente). Falta: medir con Lighthouse (Chrome lo trae en F12 → Lighthouse, sin descargar nada) y probar la app instalada en un celu.
19. [ ] **Rate limiting**: con `workers.dev` no se puede. Los archivos estáticos de Workers son gratis y sin límite, y Cloudflare frena los DDoS solo, así que hoy no hay costo ni riesgo de quedarse sin cuota. Con dominio propio, sumar una regla de rate limiting de Cloudflare (el plan gratis trae una).
20. [ ] **Testeo de punta a punta**. Escritorio hecho el 11/11/2026 (commit pendiente):
    - Recorrido automático del build local: las 37 URLs del sitemap y los 44 links internos responden; `lang`, `title`, `description`, canonical y un `h1` en cada página; JSON-LD válido; las 3.017 ofertas aparecen una sola vez, en orden de descuento y con el mismo nombre, precio, precio anterior y % que `data/`; `buscar.json`, `hoy.csv`, `datos/<cadena>.json` y `actualizado.json` coinciden; `llms.txt`, manifest e íconos sin links rotos; headers de seguridad presentes.
    - En Chrome: sin errores de consola en portada, cadena, preguntas, datos, términos y 404; búsqueda, filtros, guardados, aviso de fecha y offline ya probados; botón de instalar del header; ancho de 390 px sin scroll horizontal.
    - Links a las cadenas: Carrefour 6/6 abren el producto; Coto abre el producto con el mismo precio (su web arma la página en el navegador, revisado a mano).
    - Precios contra las cadenas: Carrefour 15/15, Coto 25/25, DAR 7/7 (11/11/2026).
    - Falta: el celu real (instalar la app, sin señal), que lo prueba el mantenedor.
    Recién después: estilos.
21. [x] **Imágenes de los productos**: miniaturas de Carrefour (versión de 160 px que arma VTEX) y Coto (foto "medium"), cargadas por el navegador desde el servidor de cada cadena, con carga diferida y sin referer. DAR sin fotos. En las páginas y en los resultados de búsqueda (`buscar.json` suma la miniatura como 9.º campo). CSP abierta solo a esos dos dominios; Privacidad y Términos actualizados. Probado en Chrome: cargan en Carrefour, Coto y la búsqueda, DAR sin fotos, sin errores de consola, 390 px sin scroll horizontal. (commit pendiente)

## Lanzamiento (cuando el MVP esté terminado)

El repo es **privado** hasta terminar el MVP. Al lanzarlo:

- [ ] **Antes de hacer público el repo, limpiar el historial**: la dirección personal que figuraba como sucursal de DAR está en `config/stores.toml` y en `data/offers/dar.json` de los commits viejos (desde `c65fdc1`). Opciones: reescribir el historial con `git filter-repo` y force-push (lo corre el mantenedor a mano), o publicar un repo nuevo sin historial. Revisar también que no haya otros datos personales (`git log -p -S "<texto>"`).
- [ ] Hacer público el repo.
- [ ] Agregar `CONTRIBUTING.md` (en español: cómo correrlo, cómo sumar una cadena o corregir una sucursal, reglas de cortesía con los sitios).
- [ ] Crear la rama `development` para el trabajo diario y proteger `main` (solo entra por PR).
- [ ] Sacar el primer tag de release (`v0.1.0`).
- [ ] Poner la URL del repo en el User-Agent (`config/stores.toml`) y en el pie de la web.

## Pendientes y dudas abiertas

- [ ] Nombre definitivo y dominio propio (opcional, mejor para SEO). Si se cambia, actualizar `url` en `config/site.toml` y el link del README.
- [ ] Al proteger `main` en el lanzamiento, el workflow ya no va a poder pushear `data/` directo. Opciones: permitir el bot en la regla, o mover los datos a una rama `data`.
- [ ] Íconos definitivos (van con los estilos, al final).
- [ ] Filtros por categoría en la web: Carrefour se scrapea por categoría y Coto trae `allAncestors`, pero `Offer` todavía no guarda la categoría.
- [ ] Coto: guardar los descuentos por medio de pago (`product.dtoDescuentosMediosPago`) para la página de bancos.
- [ ] Coto: confirmar que `sDisp` 1004 = disponible y 1001 = no disponible. Solo existen esos dos valores (36 tiendas en un producto: 30 con 1004, 6 con 1001). La web de Coto no deja elegir sucursal sin cuenta, así que no se pudo confirmar desde la web.
- [ ] Carrefour: promos 2x1 (`Teasers` viene vacío en Intelligent Search).
- [ ] DAR: promos bancarias y `Detalle.asp` (sin revisar).
- [ ] DAR: no tiene precio anterior; su descuento real depende del historial (etapa 2).
- [ ] Revisar términos de uso de cada sitio antes del lanzamiento (la web ya es accesible por URL, pero no está difundida).

## Problemas conocidos

- Si se pushea a `main` mientras corre el workflow, el commit de `data/` del bot es rechazado (pasó el 11/11/2026). La web queda con los datos nuevos, pero el repo no; al día siguiente se compara contra el archivo viejo. No pushear entre las 07:00 y las 07:45, o no disparar el workflow a mano mientras se trabaja.
- Coto: unas 40 ofertas de "Otras Ofertas" no traen precio regular (`"Precio Contado:"` vacío) y se publican sin descuento.

- En el navegador (sin instalar la app), la búsqueda sin señal solo anda si antes se buscó algo con conexión: el índice se guarda la primera vez que se usa.
- Una corrida completa tarda ~3,5 minutos, la mayor parte en Carrefour (pausa de 1 s entre requests, 12 categorías).
- La ventana de Chrome maximizada no se deja achicar con la extensión; para ver ancho de celular se usó un iframe de 390 px.

## Commits

Del más nuevo al más viejo. Cada commit que cambie algo importante se agrega acá.

| Commit | Fecha | Qué |
| --- | --- | --- |
| `0c125e0` | 11/11/2026 | Aviso cuando las ofertas en pantalla no son de hoy (`datos/actualizado.json`). |
| `6c3caa7` | 11/11/2026 | DAR figura solo como "DAR": sin la dirección personal en la config ni en el brief. |
| `957df38` | 11/11/2026 | Workflow con actions fijadas por SHA, `wrangler` exacto y Dependabot. |
| `71ce648` | 11/11/2026 | Contacto por mail y guardados con nombre y botón "Descartar". |
| `f6dc91a` | 11/11/2026 | Páginas de preguntas, términos y privacidad, favicon de flecha y headers de seguridad. |
| `085a52f` | 11/11/2026 | Cadena que falla se publica vacía con aviso; si fallan todas, el workflow igual publica; `datos/` sin caché de una hora; README con cómo instalarla. |
| `6fa4438` | 11/11/2026 | STATUS con el hash de `1e5ae5f`. |
| `1e5ae5f` | 11/11/2026 | Coto: precio regular desde "Precio Contado" en Otras Ofertas. Control de caídas: solo 0 ofertas es error. |
| `712261f` | 11/11/2026 | Docs: `context.md` y `AGENTS.md` alineados con el código; decisión sobre el subdominio. |
| `de3ec79` | 11/11/2026 | URL pública del sitio en `config/site.toml` y en el README. |
| `c2bbbf9` | 11/11/2026 | Sitio estático (portada, secciones por cadena, datos abiertos, SEO, `llms.txt`), PWA offline, búsqueda y guardados, filtro de categorías de Coto, control de caídas, `wrangler.jsonc` y workflow diario. |
| `c65fdc1` | 11/11/2026 | Primer commit: scrapers de Carrefour, Coto y DAR, detector, salida en `data/`, tests, docs, licencia y datos de la primera corrida. |
