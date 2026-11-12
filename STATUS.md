# STATUS: dónde está el proyecto

Leé esto primero si llegás sin contexto (persona o agente). Dice qué queremos, por dónde vamos, qué se decidió y qué falta.

**Regla:** este archivo se actualiza en cada cambio que importe. Cada tarea terminada cita el commit que la cerró (hash corto). Los commits se hacen a mano; el hash se carga después. Si algo de acá no coincide con el código o con `git log`, manda el código: corregí este archivo.

Última actualización: 12/11/2026.

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

Primera corrida por cron (11/11/2026, run `43069628912`, evento `schedule`, con `d648065`): todo OK, deploy y commit de `data/` (`11906ee`). Arrancó a las 14:35 en vez de las 07:00: GitHub atrasa los workflows programados cuando tiene carga (el 12/11 a las 10:00 todavía no había corrido). Un push a `main` no publica: el código nuevo sale en la próxima corrida.

**Próximo paso:** lo que falta para el lanzamiento (ver "Lanzamiento" más abajo).

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
| Tests | ✅ 24 tests con respuestas guardadas | `tests/fixtures/`, `#[cfg(test)]` en cada módulo |
| Control de caídas | ✅ Si una cadena trae 0 ofertas (y ayer tenía), se toma como error y se conserva el archivo anterior. Las bajas grandes solo quedan en el log | `check_volume` en `src/main.rs` |
| Sitio estático (HTML, SEO) | ✅ Portada, una sección paginada por cadena, datos abiertos, 404, JSON-LD, sitemap, robots, llms.txt | `src/site.rs`, `templates/`, `config/site.toml` |
| PWA (manifest, service worker, aviso de instalación) | ✅ Chrome la ofrece para instalar; offline con precache | `web/manifest.webmanifest`, `web/sw.js`, `web/app.js` |
| Búsqueda y guardados | ✅ Búsqueda sin tildes sobre `datos/buscar.json`; guardados en `localStorage` | `web/app.js` |
| Íconos | ✅ Flecha de cursor blanca sobre celeste `#0369A1`, más `favicon.svg` | `web/icons/` |
| Estilos | ✅ Paleta "Celeste Rosario", Inter, celu primero, grilla en escritorio, modo oscuro según el dispositivo | `web/styles.css`, `web/fonts/` |
| Páginas legales | ✅ Preguntas frecuentes, términos y privacidad. Falta el canal de contacto y la revisión legal | `templates/preguntas.html`, `terminos.html`, `privacidad.html` |
| Deploy Cloudflare | ✅ Publicado (manual con `wrangler`) | `wrangler.jsonc` |
| GitHub Actions diario | ✅ Corrida manual y por cron OK (11/11). El cron puede salir con horas de atraso | `.github/workflows/daily.yml` |
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
| 10/11/2026 | Nada de estilos, fuentes ni colores hasta que la app ande y esté testeada (cumplido: estilos desde el 11/11/2026, ver más abajo) | Prioridad: que funcione. |
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
| 11/11/2026 | Costo cero en Cloudflare: plan Workers Free, Worker solo de archivos estáticos (sin `main` ni `run_worker_first`), sin medio de pago cargado | Los pedidos a archivos estáticos son gratis y sin límite, sin importar el tráfico. El plan gratis no cobra nunca: si un día hubiera código en el Worker y se pasara de 100.000 pedidos por día, el sitio daría error 1027 hasta medianoche UTC, pero no genera deuda. Solo se cobra si se pasa al plan pago ($5/mes). Las notificaciones push previstas para más adelante necesitarían código en el Worker: revisar esto antes. |
| 11/11/2026 | Estilos con la paleta de Cloudflare: Tangerine `#F6821F` de acento, Ruby para descuentos de 40% o más, Mango para promos, Lemon para el aviso de fecha; links `#C2410C` (claro) y Mango (oscuro). Texto sobre naranja siempre casi negro (reemplazada el mismo día por "Celeste Rosario", ver más abajo) | Pedido del mantenedor (plan aprobado). En Cloudflare el blanco sobre naranja da 2,58:1 y no pasa WCAG; con casi negro da 6,67:1. Todos los pares de texto pasan 4,5:1 en los dos temas. |
| 11/11/2026 | Inter alojada en el sitio (subconjunto latino, variable, 48 KB, licencia OFL en `web/fonts/OFL.txt`), no Google Fonts | Misma tipografía que Cloudflare sin terceros: no toca la CSP ni Privacidad y queda offline con el service worker. Bajada de `@fontsource-variable/inter@5.2.8` (jsDelivr) con OK del mantenedor. |
| 11/11/2026 | Celu primero; en escritorio grilla de 2 a 4 tarjetas y filtros en columna; modo oscuro según el dispositivo (`prefers-color-scheme`) | Decisiones del mantenedor. En el celu los filtros quedan detrás de "Filtros (n)". |
| 11/11/2026 | Botón de modo claro/oscuro en el header; la elección se guarda en el dispositivo (`localStorage`) y la aplica `theme.js` antes de pintar; sin elección, manda el dispositivo | Pedido del mantenedor. `theme.js` va sin `defer` para que no parpadee el tema equivocado (la CSP no deja scripts inline). Privacidad lo menciona. |
| 11/11/2026 | Se sacaron señales de "web vibecodeada" (según checklists de The Fountain Institute, 925 Studios y la "Anti AI slop ban list"): borde de color a la izquierda en los recuadros, el mismo radio en todo (ahora por rol: botones píldora, tarjetas 10, campos 6, etiquetas 4), texto repetido (chip "35%Dto" junto al -35%, "Oferta" en todo DAR, link duplicado "Ver las N →"), recuadros punteados para todo, salto de texto al cargar la fuente (fallback con métricas de Inter) y estados de deshabilitado/presionado | Que el sitio no parezca hecho en automático. Se mantiene Inter (también figura en esas listas) porque fue una decisión del mantenedor. Las flechas de paginación y la de "sitio externo" quedan: son convención, no figuran en las listas. |
| 11/11/2026 | Botón "Limpiar filtros" (vuelve a todas las cadenas, cualquier descuento, todas las promos y el orden por defecto; conserva el texto buscado) y combos sin la flecha nativa (pegada al borde) con una propia con margen | Pedido del mantenedor. |
| 11/11/2026 | Fotos de Carrefour y Coto desde sus servidores (sin copiarlas), DAR sin fotos | Decisión del mantenedor. No se copian las fotos (son de las cadenas y marcas); se cargan desde la cadena con `loading="lazy"` y `referrerpolicy="no-referrer"`. DAR sirve las fotos desde su propio servidor chico. El servidor de imágenes de Coto tiene `robots.txt` que pide a los bots no entrar: el scraper nunca baja imágenes, las carga el navegador de cada persona. Costo: las cadenas ven la IP de quien mira (dicho en Privacidad) y sin señal no hay fotos. |
| 11/11/2026 | `discount_pct` truncado en vez de redondeado | Nunca mostrar más descuento que el real; con redondeo, un 35,5% salía "-36%" al lado del "35%Dto" de Coto. Se ve desde la próxima corrida (se calcula al scrapear). |
| 11/11/2026 | Botón "¡Instalá la app!" en el header en vez de la sección "Instalala en el celu" | Pedido del mantenedor. Abre el diálogo de Chrome si está disponible; si no, muestra cómo hacerlo. Desaparece con la app instalada. |
| 11/11/2026 | `buscar.json` fuera del precache; la app instalada lo baja en segundo plano | Eran 108 KB de los ~150 KB de la primera visita, aunque la persona no buscara nada. Quien usa el sitio sin señal en el súper es quien lo instala. |
| 11/11/2026 | Filtros en la búsqueda de la portada (sobre `buscar.json`), no dentro de las páginas de cadena | Las páginas de cadena son HTML fijo de a 100 para buscadores; filtrar ahí solo vería esas 100. La búsqueda tiene todas las ofertas. Las páginas de cadena enlazan a la búsqueda con la cadena elegida. (Reemplazada el mismo día: ahora cada página de cadena tiene sus filtros, ver más abajo.) |
| 11/11/2026 | El formulario de búsqueda ya no usa `onsubmit` inline; el envío lo maneja `app.js` | La CSP bloquea los handlers inline: con Enter se recargaba la página. |
| 11/11/2026 | Aviso de fecha en la página en vez de cambiar la estrategia de caché | El caché sirve sin señal en el súper y no hay que perderlo. Lo que faltaba era decir de qué día son las ofertas cuando no son de hoy. "Todavía no están las de hoy" también cubre el rato antes de la corrida y una corrida fallida. |
| 11/11/2026 | El workflow no hace `pull --rebase` antes de pushear `data/` | Si se pushea algo mientras corre, que falle y quede registrado; se aprende de eso. Los datos de ese día igual quedan publicados en la web. |
| 11/11/2026 | Contraste de controles: bordes de botones, chips y foco con `--line` (hoy el celeste `#0369A1` en claro y `#38BDF8` en oscuro), bordes de campos y botones secundarios con `--control` (`#8A8A93` claro, `#71717A` oscuro) y placeholder con `--placeholder`. `--border` queda solo para tarjetas y divisores | Tangerine sobre blanco da 2,58:1 y los grises de borde 1,2:1: no llegaban al 3:1 que pide WCAG 1.4.11 para identificar un control. Auditado con un script en Chrome sobre todas las páginas, claro y oscuro, escritorio y 390 px: todo texto 4,5:1 y todo borde de control 3:1. |
| 11/11/2026 | Búsqueda vacía con atajos: tarjetas "50% o más", "40% o más", "2x1 y 3x2" y "2da unidad" con la cantidad de ofertas del día, y "Probá con" palabras comunes (yerba, leche...) con su cantidad. Se filtran en la página, sin recargar. Los conteos se calculan al generar el sitio (`quick_counts` en `src/site.rs`, misma regla que el filtro de `app.js`); un atajo o palabra con 0 ofertas no se muestra | Pedido del mantenedor: el texto solo ("Buscá un producto o usá los filtros...") no llamaba la atención. Calcularlo en el build evita bajar `buscar.json` en la primera visita. |
| 11/11/2026 | Preguntas, términos y privacidad con diseño propio sin cambiar el texto: título grande, preguntas en tarjetas numeradas (dos columnas en escritorio), términos en un panel con una fila por cláusula (título a la izquierda en escritorio), privacidad con "Qué no hacemos" como lista de checks y el resto en tarjetas | Pedido del mantenedor. Verificado que el texto de las tres páginas es idéntico al anterior (comparación del texto plano del build viejo y el nuevo). |
| 11/11/2026 | Paleta "Celeste Rosario" en vez de la de Cloudflare: celeste `#0369A1` de acento (botones, descuento, logo, íconos, `theme-color`), rojo `#BE123C` para descuentos de 40% o más, amarillo `#FDE047` para promos y el aviso de fecha, celeste claro `#E0F2FE` en un atajo. Links y bordes `#0369A1` en claro y `#7DD3FC` / `#38BDF8` en oscuro. Texto blanco sobre celeste (5,9:1) y rojo (6,3:1), casi negro sobre amarillo | Al mantenedor no le gustaba el naranja con texto negro, y con naranja el texto blanco no se lee (2,58:1). Elegida entre cuatro opciones (celeste, verde, azul y rojo, violeta) probadas sobre el sitio real: el celeste de la bandera le da identidad local (Rosario, Cuna de la Bandera). El botón principal lleva borde `--line`: en oscuro el relleno celeste solo daba 2,99:1 contra el fondo. Íconos PNG recoloreados con un script sin dependencias que conserva el antialias. Auditado en Chrome: 11 páginas, claro y oscuro, escritorio y 390 px, todo texto 4,5:1 y todo borde de control 3:1. |
| 11/11/2026 | Íconos SVG en vez de caracteres: la flecha de "ver en ..." (antes ↗), las de Anterior/Siguiente (antes ← →) y la de los atajos (antes → en el CSS). Macros en `templates/icons.html` y los mismos trazos en `ICONS` de `app.js`. Se dejan como texto el % y el ⋮ de las instrucciones de instalación (es el ícono del menú del navegador que la persona tiene que buscar) | Pedido del mantenedor. Los caracteres dependen de la fuente y se ven distintos en cada dispositivo; los SVG heredan el color y el tamaño del texto. |
| 11/11/2026 | Botón flotante "Volver arriba" (flecha), visible recién después de bajar una pantalla y media; respeta "reducir movimiento" y deja el foco en el logo. El pie tiene 80 px de aire abajo para que el botón no tape los últimos links. En el celu, el botón "Filtros" tiene 8 px de margen abajo | Pedido del mantenedor: las listas de 100 ofertas son largas en el celu. |
| 11/11/2026 | Filtros dentro de cada página de cadena (mismo formulario que la portada, `templates/search_form.html`, con la cadena fija y sin las casillas de cadena) en vez del botón "Filtrar las ofertas de ..." que llevaba al inicio. Sin filtros se ve la lista fija de 100 con la paginación (lo que leen los buscadores); con algún filtro, o cambiando el orden, se busca sobre todas las ofertas de esa cadena en `buscar.json`. "Limpiar filtros" vuelve a la lista de la página | Pedido del mantenedor: al tocar el botón perdía la página de la cadena. Así se mantiene el HTML fijo para SEO y los filtros ven todas las ofertas, no solo las 100 de la página. |
| 11/11/2026 | Filtro de rango de precio ("Desde" y "Hasta", en pesos enteros) en la portada y en las páginas de cadena; queda en la URL (`?desde=1000&hasta=3000`), se aplica mientras se escribe y, si "Desde" es mayor que "Hasta", se invierten (cambiado el mismo día: ahora es un error de validación, ver más abajo) | Pedido del mantenedor. Verificado en Chrome contra `buscar.json`: Carrefour de $1.000 a $3.000 da 628 ofertas en los dos cálculos. |
| 11/11/2026 | Validaciones de los filtros. Precio: pesos enteros escritos como "1500", "1.500" o "$ 1.500" (campo de texto numérico, no `type=number`, que leía "1.500" como 1,5), hasta $10.000.000, y "Desde" menor o igual que "Hasta"; si no, mensaje en rojo debajo de los campos, borde rojo y `aria-invalid`, y el filtro de precio no se aplica ni va a la URL. Al salir del campo se reescribe con puntos ("1500" pasa a "1.500"). Búsqueda: máximo 80 caracteres y aviso "Escribí al menos 2 letras para buscar." si solo hay palabras de una letra. URL: se ignoran cadenas, descuentos, promos, órdenes y precios que no existen o no son válidos; si ninguna cadena es válida, quedan todas | Pedido del mantenedor. Los mensajes van en regiones `aria-live` que nunca se ocultan con `display: none` (si no, algunos lectores de pantalla no los anuncian). Color de error `--danger` (`#B91C1C` claro, `#FCA5A5` oscuro): 6,5:1 y 8,6:1. Probado en Chrome: letras, coma decimal, negativo, mayor al máximo, rango invertido, "$ 3.000", URL con valores inválidos. |
| 11/11/2026 | Botones "Anterior" y "Siguiente" de la paginación con rutas relativas (`/coto/pagina/2/`); las absolutas (`prev_url`, `next_url`) quedan solo en `<link rel="prev/next">` del `<head>` | Bug: los botones usaban la URL absoluta de producción, así que en local (o en cualquier copia que no sea la publicada) llevaban al sitio publicado. Test `pager_links_stay_on_the_same_site`. |
| 11/11/2026 | El subdominio `workers.dev` lleva el nombre del mantenedor y se deja así por ahora (excepción a "sin datos personales") | Cambiarlo no aporta hasta tener dominio propio; se reemplaza junto con el dominio. |
| 11/11/2026 | Promos por cantidad ("Llevando N"): el precio dice "c/u llevando N" y el chip pierde el "(Llevando N)" (queda "2x1", "70% 2da"; "30%Dto (Llevando 6)" ya no muestra chip). Regla en `promo_units`/`promo_chip` (`src/site.rs`) y `promoUnits`/`promoChip` (`app.js`). "Cómo funciona" lo explica | Revisión de diseño: "$599,02 antes $1.199" parecía el precio de góndola, y en la góndola dice $1.199. Es el precio por unidad llevando 2. |
| 11/11/2026 | Portada: 6 ofertas por cadena (antes 20; 6 llena parejo la grilla de 2 y de 3 columnas) y una por "tipo de producto" (primera palabra del nombre) para no repetir variantes. DAR se titula "Algunas ofertas de DAR" | Carrefour mostraba 3 encendedores Bic y 6 jabones Skip, Coto 5 gelatinas y DAR (sin descuentos, orden alfabético) 13 acondicionadores. |
| 11/11/2026 | Búsqueda: orden "Recomendado" por defecto (con texto: primero los nombres que empiezan con la palabra, después palabra entera, después en cualquier parte; desempata el descuento; sin texto, mayor descuento). "Mayor descuento" pasa a ser una opción (`?orden=descuento`) | Buscando "leche" salían primero helado, chocolate y jabón. |
| 11/11/2026 | Si falla la descarga de `buscar.json` no se guarda el error (antes la búsqueda quedaba rota hasta recargar); se muestra "Cargando las ofertas…" en la primera búsqueda y "Reintentar" si falla. Búsqueda sin resultados con sugerencia y "Borrar la búsqueda y los filtros" | Revisión de diseño: perder señal una vez en el súper rompía la búsqueda. |
| 11/11/2026 | Accesibilidad: link "Saltar al contenido", nombre de cada oferta como `h3`, solo el resumen ("86 ofertas") es región `aria-live` (antes toda la lista), "Guardar" dice qué producto, el badge se lee "-50% de descuento", el botón de tema ya no usa `aria-pressed` (su texto ya dice la acción). En pantallas táctiles, "Guardar", "ver en ..." y el botón de tema miden 44 px | Revisión de diseño: el lector de pantalla leía decenas de tarjetas en cada cambio y "Guardar" se repetía 100 veces sin contexto. |
| 11/11/2026 | Cambios de filtro con combos, casillas o atajos crean una entrada en el historial (Atrás los deshace); escribir solo reemplaza la actual. En el celu, botón "Ver N ofertas" al final de los filtros | Antes Atrás salía de la página, y con los filtros abiertos los resultados quedaban abajo sin aviso. |
| 11/11/2026 | Detalles: acciones de las tarjetas alineadas por fila (un `margin-top: auto` que pisaba `.card-body p`), 404 sin canonical, con `noindex`, sin "Inicio" marcado y con un botón por cadena, pie abajo en páginas cortas, preguntas frecuentes en grilla (se leen 01, 02, 03... por fila), `text-wrap: pretty/balance`, fotos atenuadas en modo oscuro, atajos de la portada menos pesados que la búsqueda, pestañas que se desvanecen a la derecha si hay más, "Página X de Y" solo en el paginador | Revisión de diseño (skill impeccable, 27/40). Quedan sin cambiar, por decisión del mantenedor: los atajos 40% y 50% se solapan (es común en atajos "o más") y los combos filtran con cada flecha del teclado (solo en escritorio con el combo cerrado; demorarlo haría más lento el mouse). |
| 11/11/2026 | "Probá con" pasa de 8 a 19 palabras, ordenadas como la canasta básica: yerba, leche, carne, huevos, fideos, arroz, harina, azúcar, aceite, queso, galletitas, café, gaseosa, cerveza, papel higiénico, detergente, pañales, alimento para perro y alimento para gato. El conteo usa la misma regla que la búsqueda (todas las palabras, en cualquier orden: `matches_search`), así sirven las frases. Quedan afuera "pan", "vino", "pollo" y "té" | Pedido del mantenedor: sumar lo que más se consume en Argentina, incluida la comida para mascotas. Esas cuatro coinciden por texto con otros productos ("panales", "vinagre", alimento para perro sabor pollo, casi cualquier nombre con "te"). Verificado en Chrome: los 19 conteos coinciden con los resultados de la búsqueda. |
| 11/11/2026 | Portada en escritorio: la columna de filtros queda fija a la izquierda y todo lo demás (atajos o resultados, guardados, mejores ofertas, cómo funciona) va a la derecha. En el celu no cambia | Elegido por el mantenedor entre tres opciones (también: filtros en una barra arriba, o dejarlo). Antes la columna de filtros dejaba un hueco a la derecha del panel de atajos. |
| 11/11/2026 | Celu: al bajar se esconde la fila de pestañas y queda fija solo la barra con la marca y los botones; al subir vuelven. La fila se desliza por debajo de la barra (el header es transparente y deja pasar los clics), así la página no se mueve. Mismo orden de foco que antes (marca, pestañas, botones) | Elegido por el mantenedor (también: header no fijo, o una fila con menú). El header fijo ocupaba ~95 px de pantalla en el celu. |
| 12/11/2026 | El bot sigue commiteando `data/` en `main` cuando se proteja: excepción en el ruleset con una deploy key de escritura | Mantiene el workflow y el historial diario en `main` sin otra rama. `GITHUB_TOKEN` no puede ir en la lista de excepciones de un ruleset; una deploy key o una GitHub App sí. |
| 12/11/2026 | El historial se limpia con `git filter-repo` en vez de publicar un repo nuevo | Conserva el historial de commits y de los datos diarios; solo se reemplaza la dirección. Después del force-push se pide a GitHub Support que purgue los commits viejos. |

## Próximos pasos

En orden. Cuando cierres una, marcala y poné el commit.

1. [x] **Sitio estático**: comando `build`, portada, una sección paginada por cadena, datos abiertos (`datos/<cadena>.json`, `datos/hoy.csv`), 404. HTML con las ofertas escritas, `title`, `description`, canonical, Open Graph, `lang="es-AR"` y JSON-LD. Sin estilos. (`c2bbbf9`)
2. [x] **SEO técnico**: `sitemap.xml`, `robots.txt` abierto a buscadores y bots de IA, `llms.txt`. (`c2bbbf9`)
3. [x] **PWA**: manifest, `sw.js` offline, botón "Instalar" y texto para iPhone/Android; se oculta si ya está instalada. (`c2bbbf9`)
4. [x] **JS del navegador**: búsqueda y guardados en `localStorage`. (`c2bbbf9`)
5. [x] **Deploy**: `npx wrangler@4 deploy` el 11/11/2026, Worker `ofertas-rosario`. URL en `config/site.toml`; todas las rutas responden y Cloudflare comprime (buscar.json: 1,4 MB → 248 KB). (`de3ec79`)
6. [x] **GitHub Actions**: secrets `CLOUDFLARE_API_TOKEN` (token de cuenta con permiso Account · Workers Scripts · Edit) y `CLOUDFLARE_ACCOUNT_ID` cargados el 11/11/2026. Corrida manual del 11/11 completa: las tres cadenas responden desde GitHub, deploy y commit de `data/` andan (`c287d33`). Cron confirmado: el `schedule` disparó solo el 11/11/2026 (run `43069628912`, commit `11906ee`), con horas de atraso.
7. [x] **Control de caídas**: 0 ofertas = error (al principio era "menos de la mitad que ayer"); el workflow abre un issue si alguna cadena falla. (`c2bbbf9`, ajustado después)
8. [x] **README**: cómo instalarla en el celu y datos abiertos, hecho (`085a52f`). Capturas en `docs/capturas/` (12/11/2026): búsqueda en escritorio a 1280 px en claro, portada en el celu en claro y Coto en el celu en oscuro, a 390 px. Sacadas con Chrome del build local con los datos del 12/11, iguales a los publicados (producción no deja meterse en un iframe). (`1f55d8b`)
9. [ ] **Alta en buscadores** (la hace el dueño del dominio): Google Search Console (propiedad "Prefijo de URL", verificación por archivo HTML en `web/`) y Bing Webmaster (importar desde Google). **Esperar** a la revisión de términos de las cadenas y al dominio propio.
10. [x] **Páginas legales e informativas**: `/preguntas-frecuentes/`, `/terminos/` (aviso legal: sin relación con las cadenas, precios informativos, sin garantías, cómo se leen los datos, pedidos de retiro, ley argentina) y `/privacidad/` (sin cookies ni analítica; verificado que Cloudflare no pone cookies). Links en el pie y en el sitemap. 404 verificado en producción. (`f6dc91a`, `71ce648`)
11. [x] **Canal de contacto**: un mail en `contact` de `config/site.toml`, con link `mailto:` en preguntas, términos y privacidad. Sirve para los pedidos de corrección o retiro de las cadenas. (`f6dc91a`, `71ce648`)
12. [ ] **Revisión legal**: los textos de términos y privacidad los escribió un agente, no un abogado. Que los revise alguien con matrícula antes del lanzamiento, junto con los términos de uso de cada cadena.
13. [x] **Favicon**: flecha de cursor blanca sobre negro (`web/icons/favicon.svg` + PNG 192/512/maskable). Provisorio hasta los estilos. (`f6dc91a`, `71ce648`)
14. [x] **Seguridad, arreglos rápidos**: headers en `web/_headers` (CSP que solo permite el propio sitio, `frame-ancestors 'none'`, Permissions-Policy, COOP, HSTS) y links a las cadenas solo si son `https://`. Probado en local con un servidor que aplica `_headers`: sin errores de CSP. En producción (12/11/2026): los seis headers llegan en todas las rutas, `sw.js` sin caché, `/datos/` con CORS, sin `Set-Cookie`; en Chrome, portada, búsqueda y página de Carrefour sin errores de consola ni violaciones de CSP, con las fotos de Carrefour y Coto cargando. (`f6dc91a`, `71ce648`)
15. [x] **Seguridad del workflow**: actions fijadas por SHA (`actions/checkout` v7.0.1, que ya corre en Node 24; `dtolnay/rust-toolchain` stable; `Swatinem/rust-cache` v2.9.3), `wrangler@4.155.0` exacto y Dependabot semanal para Cargo y Actions (`.github/dependabot.yml`). `wrangler` se actualiza a mano: Dependabot no ve `npx`. Visto en corridas reales: cron del 11/11 y manual del 12/11 (run `43223515281`), todos los pasos OK. (`957df38`)
16. [x] **Datos viejos en caché**: cada página lleva la fecha de sus datos (`body[data-updated]`) y `app.js` la compara con `/datos/actualizado.json`, que el service worker nunca cachea. Avisa "Hay ofertas más nuevas (Recargar)", "Sin conexión: estas ofertas son del DD/MM", o "Todavía no están las ofertas de hoy". Si todo es de hoy, nada. Vuelve a chequear al reconectarse o al volver a la app. Probados los cuatro casos en Chrome. (`0c125e0`)
17. [x] **Filtros y orden (UX)**: la búsqueda de la portada pasó a "Buscar y filtrar": producto, cadena (casillas), descuento mínimo (10 a 50%), tipo de promo (precio rebajado, 2x1 y 3x2, 2da unidad, llevando varias) y orden (descuento, precio, nombre). Se aplican al cambiar, sin texto también; resultados de a 50 con "Mostrar más"; los filtros quedan en la URL (`/?cadena=coto&min=40`). Cada página de cadena tiene "Filtrar las ofertas de ..." (después: los filtros están en la propia página de cadena, y se sumó el rango de precio). Probado en Chrome, también a 390 px. Por categoría falta guardar la categoría en `Offer` (ver Pendientes). (`4ad3a8f`)
18. [ ] **Performance**: medido en producción (comprimido): portada 7,5 KB, página de cadena ~11 KB, `app.js` 2,6 KB, `buscar.json` 108 KB (583 KB sin comprimir). Hecho: `buscar.json` ya no se precachea en la primera visita (era ~70% de lo que bajaba); se guarda al buscar por primera vez, y la app instalada lo baja en segundo plano para buscar sin señal. Probado en Chrome: primera visita sin el índice, y búsqueda sin servidor después de haber buscado una vez (`d31eb51`). Falta: medir con Lighthouse (Chrome lo trae en F12 → Lighthouse, sin descargar nada) y probar la app instalada en un celu.
19. [ ] **Rate limiting**: con `workers.dev` no se puede. Los archivos estáticos de Workers son gratis y sin límite, y Cloudflare frena los DDoS solo, así que hoy no hay costo ni riesgo de quedarse sin cuota. Con dominio propio, sumar una regla de rate limiting de Cloudflare (el plan gratis trae una).
20. [ ] **Testeo de punta a punta**. Escritorio hecho el 11/11/2026 (`146bf66`, `cc9992b`):
    - Recorrido automático del build local: las 37 URLs del sitemap y los 44 links internos responden; `lang`, `title`, `description`, canonical y un `h1` en cada página; JSON-LD válido; las 3.017 ofertas aparecen una sola vez, en orden de descuento y con el mismo nombre, precio, precio anterior y % que `data/`; `buscar.json`, `hoy.csv`, `datos/<cadena>.json` y `actualizado.json` coinciden; `llms.txt`, manifest e íconos sin links rotos; headers de seguridad presentes.
    - En Chrome: sin errores de consola en portada, cadena, preguntas, datos, términos y 404; búsqueda, filtros, guardados, aviso de fecha y offline ya probados; botón de instalar del header; ancho de 390 px sin scroll horizontal.
    - Links a las cadenas: Carrefour 6/6 abren el producto; Coto abre el producto con el mismo precio (su web arma la página en el navegador, revisado a mano).
    - Precios contra las cadenas: Carrefour 15/15, Coto 25/25, DAR 7/7 (11/11/2026).
    - Falta: el celu real (instalar la app, sin señal), que lo prueba el mantenedor.
    Recién después: estilos.
21. [x] **Imágenes de los productos**: miniaturas de Carrefour (versión de 160 px que arma VTEX) y Coto (foto "medium"), cargadas por el navegador desde el servidor de cada cadena, con carga diferida y sin referer. DAR sin fotos. En las páginas y en los resultados de búsqueda (`buscar.json` suma la miniatura como 9.º campo). CSP abierta solo a esos dos dominios; Privacidad y Términos actualizados. Probado en Chrome: cargan en Carrefour, Coto y la búsqueda, DAR sin fotos, sin errores de consola, 390 px sin scroll horizontal. (`cc9992b`)
22. [x] **Estilos y diseño responsive** (plan aprobado el 11/11/2026): `web/styles.css` con la paleta "Celeste Rosario" (al principio la de Cloudflare, ver Decisiones), Inter alojada en el sitio, header fijo con pestañas, tarjetas de oferta (foto, precio grande, cartel de descuento, chip de promo), filtros desplegables en el celu y en columna en escritorio, grilla de 2 a 4, paginación con botones, columna de lectura en las páginas de texto, modo oscuro. Íconos y `theme_color` en celeste `#0369A1`. Primera visita: +52 KB (CSS 3 KB + fuente 48 KB, comprimidos). Probado en Chrome: escritorio (oscuro) y 390 px (claro y oscuro, sin scroll horizontal), búsqueda, filtros abiertos, cadena con paginación, preguntas; sin errores de consola; contraste de todos los pares 4,5:1 o más; recorrido automático sin problemas. Después: botón de modo claro/oscuro, "Limpiar filtros", flecha de los combos y limpieza de señales de web vibecodeada (ver Decisiones); probado en Chrome (el tema se guarda al recargar y cambiar de página, limpiar filtros, header de una línea a 360 y 390 px). Después: contraste de bordes de controles, atajos en la búsqueda vacía, diseño de preguntas, términos y privacidad, y el botón de instalar ya no dice "¡Instalá la app !" con espacio (ver Decisiones); probado en Chrome en claro y oscuro, escritorio y 390 px, sin scroll horizontal ni errores de consola, con los conteos de los atajos iguales a los resultados de la búsqueda. Después: arreglos de la revisión de diseño (ver Decisiones, 11/11/2026: precio por unidad, portada de 8 por cadena, relevancia, reintento, accesibilidad, historial); probado en Chrome en escritorio y en un iframe de 390 px, claro y oscuro, sin errores de consola, con la búsqueda fallando y reintentando, y 0 de 34 filas desalineadas en Coto. Falta: verlo en un celu real. (`ceb0b81`, `3500fc3`)

## Lanzamiento (cuando el MVP esté terminado)

El repo es **privado** hasta terminar el MVP. Al lanzarlo:

- [ ] **Antes de hacer público el repo, limpiar el historial** con `git filter-repo` (decidido el 12/11/2026). Revisado el 12/11/2026: la dirección personal que figuraba como sucursal de DAR está en `config/stores.toml`, `docs/brief.md` y `data/offers/dar.json`, desde `c65fdc1` hasta los commits del bot del 11 y 12/11 (en `data/offers/dar.json`). `HEAD` ya está limpio. Los autores usan el mail `noreply` de GitHub; no hay rutas locales; los logs de Actions, el issue #1 y los PRs (ninguno) no la tienen. El mail de contacto de `config/site.toml` queda a propósito: se publica en la web. Todo esto lo corre el mantenedor a mano:
    1. Elegir un momento fuera de la corrida diaria y pausar el workflow (`gh workflow disable daily.yml`).
    2. Instalar `git-filter-repo` (en WSL: `sudo apt install git-filter-repo`).
    3. Clonar de cero en una carpeta aparte (filter-repo pide un clon recién hecho) y correr `git filter-repo --replace-text <archivo>`. El archivo va **fuera del repo** y tiene dos líneas: la sucursal con la dirección `==>DAR`, y la dirección sola `==>(dirección quitada)`.
    4. Verificar en el clon: buscar la dirección con `git grep -i <calle> $(git rev-list --all)` (no tiene que salir nada) y que la cantidad de commits sea la misma.
    5. Los hashes cambian todos: pasar los de `STATUS.md` a los nuevos con el mapa que deja filter-repo en `.git/filter-repo/commit-map`, y commitear.
    6. `git remote add origin <url>` y `git push --force origin main`.
    7. Pedir a GitHub Support que borre las vistas en caché y los commits viejos: después del force-push siguen abriéndose por su hash. Esperar la confirmación antes de hacer público el repo.
    8. Volver a clonar la copia de trabajo. **No hacer `git pull` en la copia vieja**: mezclaría el historial viejo y un push lo volvería a subir.
    9. Reactivar el workflow (`gh workflow enable daily.yml`).
- [ ] Hacer público el repo.
- [x] Agregar `CONTRIBUTING.md` (en español: cómo correrlo, cómo sumar una cadena o corregir una sucursal, reglas de cortesía con los sitios). (`2e8c483`)
- [ ] Crear la rama `development` para el trabajo diario y proteger `main` (solo entra por PR), con una excepción para el bot de los datos (ver Decisiones, 12/11/2026): crear una deploy key con permiso de escritura, cargar la clave privada como secret, que el `actions/checkout` del workflow la use (`ssh-key`) y sumar la deploy key a la lista de excepciones del ruleset. Probar con una corrida manual que el commit de `data/` entre.
- [ ] Sacar el primer tag de release (`v0.1.0`).
- [ ] Poner la URL del repo en el User-Agent (`config/stores.toml`) y en el pie de la web. Preparado (`2e8c483`): el pie muestra "Código fuente" cuando `repo` de `config/site.toml` tiene valor, y el User-Agent nuevo está comentado en `config/stores.toml`. Quedan vacíos hasta que el repo sea público, porque el sitio se publica solo y el link daría 404. Al lanzar: completar `repo`, cambiar el User-Agent y actualizar `docs/context.md`.

## Pendientes y dudas abiertas

- [ ] Nombre definitivo y dominio propio (opcional, mejor para SEO). Si se cambia, actualizar `url` en `config/site.toml` y el link del README.
- [x] Al proteger `main` en el lanzamiento, el workflow ya no va a poder pushear `data/` directo. Decidido el 12/11/2026: excepción para el bot con una deploy key (ver Lanzamiento).
- [x] Íconos definitivos: flecha blanca sobre celeste `#0369A1` (11/11/2026; antes Tangerine).
- [x] Capturas del README (12/11/2026, ver paso 8).
- [ ] Filtros por categoría en la web: Carrefour se scrapea por categoría y Coto trae `allAncestors`, pero `Offer` todavía no guarda la categoría.
- [ ] Coto: guardar los descuentos por medio de pago (`product.dtoDescuentosMediosPago`) para la página de bancos.
- [ ] Coto: confirmar que `sDisp` 1004 = disponible y 1001 = no disponible. Solo existen esos dos valores (36 tiendas en un producto: 30 con 1004, 6 con 1001). La web de Coto no deja elegir sucursal sin cuenta, así que no se pudo confirmar desde la web.
- [ ] Carrefour: promos 2x1 (`Teasers` viene vacío en Intelligent Search).
- [ ] DAR: promos bancarias y `Detalle.asp` (sin revisar).
- [ ] DAR: no tiene precio anterior; su descuento real depende del historial (etapa 2).
- [ ] Revisar términos de uso de cada sitio antes del lanzamiento (la web ya es accesible por URL, pero no está difundida).

## Problemas conocidos

- Si se pushea a `main` mientras corre el workflow, el commit de `data/` del bot es rechazado (pasó el 11/11/2026). La web queda con los datos nuevos, pero el repo no; al día siguiente se compara contra el archivo viejo. No pushear entre las 07:00 y las 07:45, o no disparar el workflow a mano mientras se trabaja.
- Celu a 390 px: la fila de pestañas se desliza (`overflow-x: auto`) y "Datos abiertos" queda cortada unos píxeles contra el borde, sin pista de que hay más. La página no se desborda. Visto el 12/11/2026, sin tocar.
- Coto: unas 40 ofertas de "Otras Ofertas" no traen precio regular (`"Precio Contado:"` vacío) y se publican sin descuento.

- En el navegador (sin instalar la app), la búsqueda sin señal solo anda si antes se buscó algo con conexión: el índice se guarda la primera vez que se usa.
- Una corrida completa tarda ~3,5 minutos, la mayor parte en Carrefour (pausa de 1 s entre requests, 12 categorías).
- La ventana de Chrome maximizada no se deja achicar con la extensión; para ver ancho de celular se usó un iframe de 390 px.

## Commits

Del más nuevo al más viejo. Cada commit que cambie algo importante se agrega acá.

| Commit | Fecha | Qué |
| --- | --- | --- |
| `1f55d8b` | 12/11/2026 | Capturas en el README, headers y CSP verificados en producción, decisiones sobre el bot y el historial, pasos para limpiar el historial. |
| `2e8c483` | 12/11/2026 | `CONTRIBUTING.md`, link al repo en el pie (vacío hasta el lanzamiento), User-Agent de lanzamiento comentado, cron confirmado en STATUS. |
| `3500fc3` | 11/11/2026 | Arreglos de la revisión de diseño. |
| `ceb0b81` | 11/11/2026 | Estilos con la paleta Celeste Rosario, atajos en la búsqueda, rediseño de preguntas, términos y privacidad, íconos SVG, botón para volver arriba, paginado y filtros. |
| `d648065` | 11/11/2026 | Decisión de costo cero en Cloudflare (comentario en `wrangler.jsonc`) y hashes en STATUS. |
| `cc9992b` | 11/11/2026 | Fotos de Carrefour y Coto desde sus servidores; Privacidad y Términos actualizados. |
| `146bf66` | 11/11/2026 | Botón de instalar en el header, aclaración por cadena de cómo ver el mismo precio, descuento truncado, testeo de punta a punta. |
| `d31eb51` | 11/11/2026 | El índice de búsqueda ya no se precachea en la primera visita. |
| `4ad3a8f` | 11/11/2026 | Filtros por cadena, descuento y promo, y orden en la búsqueda. |
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
