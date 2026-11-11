# Ofertas Rosario — Brief del proyecto

10/11/2026 · actualizado 11/11/2026. Estado y decisiones: `STATUS.md`. Detalle técnico de cada fuente: `docs/context.md`.

## Objetivo y alcance

Una herramienta open source que todos los días junta las ofertas de los supermercados de Rosario y las muestra en una web pública. Solo Rosario: las sucursales están fijas en un archivo de configuración, no se busca por ubicación.

- **Cadenas iniciales:** DAR, Coto y Carrefour, más otras grandes de la ciudad si el proyecto crece.
- **Qué entra:** precio de oferta vs. precio normal, promos tipo 2x1 / 70% en la 2da, y descuentos bancarios por día.
- **Qué no entra (por ahora):** comparar precios de toda la canasta, compras online, cuentas de usuario, login, notificaciones.
- **Salida:** una web estática y pública, instalable en el celu como app (PWA), más un JSON/CSV diario versionado en el repo que cualquiera puede reusar.

Lo más importante que surgió al entrar a las webs: **las tres cadenas del MVP tienen precios de Rosario accesibles sin login.** DAR desde su HTML; Coto y Carrefour desde sus APIs con el código de tienda o región de Rosario. La Anónima se cubre con sus folletos por ciudad, y Jumbo con el dataset oficial SEPA. **La Gallega queda afuera:** su `robots.txt` bloquea todas las páginas de ofertas.

## Cadenas y sucursales de Rosario

Esta es la lista para el archivo de sucursales. Coto sale de su propia web (con su número interno); el resto sale de padrones provinciales y notas de prensa, así que **hay que verificarla una por una** antes de cargarla.

| Cadena | Sucursal | Dirección | Código para la API | Fuente |
| --- | --- | --- | --- | --- |
| Coto | Alto Rosario (hiper) | Av. Junín y Thedy | tienda `165` (filtro de disponibilidad online) | [coto.com.ar/sucursales](https://www.coto.com.ar/sucursales/) |
| Coto | Echesortu | Mendoza 3901 | `095` (sin catálogo online) | idem |
| Coto | Centro | Urquiza 1644 | `097` (sin catálogo online) | idem |
| Coto | Centro | Tres de Febrero 1602 | `099` (sin catálogo online) | idem |
| Coto | Fisherton (hiper) | Venezuela 114 bis | `096` (sin catálogo online) | idem |
| Carrefour | Hiper Fisherton | Av. Circunvalación 1977 bis | seller `carrefourar0018` | API de puntos de retiro de [carrefour.com.ar](https://www.carrefour.com.ar/) |
| Carrefour | Hiper Pueyrredón | Pueyrredón 747 | seller `carrefourar0268` | idem |
| Carrefour | Hiper Pellegrini | Pellegrini 3250 | a verificar | idem |
| Carrefour | Hiper Rosario Sur | Necochea 2579 | retiro vía `carrefourar0018` | idem |
| Carrefour | Hiper Village | Av. Eva Perón 5856 | a verificar | idem |
| Jumbo | Portal Rosario | Nansen 255 | solo SEPA | [La Capital](https://www.lacapital.com.ar/la-ciudad/nuevos-beneficios-los-suscriptores-la-tarjeta-blc-se-sumo-jumbo-n10219723.html) |
| DAR | DAR en tu casa (tienda online) | (dirección quitada) | — | [darentucasa.com.ar](https://www.darentucasa.com.ar/) |
| DAR | Sucursales físicas | San Luis 1131, Rioja 848, Rioja 1660, San Martín 533, Mendoza 8833, Salta y Corrientes, Córdoba y Gutemberg, Alsina y Pte. Perón, 3 de Febrero 198 | — | [Padrón Santa Fe](https://www.santafe.gob.ar/preciossantafe/listadocomercios) |
| La Anónima | Sucursal(es) ex Libertad | a verificar | folletos zona `z02` | [catálogos La Anónima](https://www.laanonima.com.ar/empresa/catalogos) |
| Todo Chen | Avellaneda (ex Micropack) | Avellaneda 441 | — | [La Capital](https://www.lacapital.com.ar/economia/renovacion-micropack-las-sucursales-avellaneda-y-arijon-tendran-nuevos-administradores-n10251946.html) |
| Todo Chen | Arijón (ex Micropack) | Arijón 565 (a verificar) | — | idem |

Carrefour queda completo con sus [cinco sucursales en Rosario](https://www.lacapital.com.ar/economia/carrefour-busca-un-comprador-su-red-super-argentina-quienes-son-los-interesados-n10206026.html): la quinta (Pueyrredón 747) apareció en la API de puntos de retiro.

La Gallega (nueve sucursales en el padrón provincial) se sacó de la lista porque su sitio no permite que se lean las ofertas de forma automática (`robots.txt`).

**Otras cadenas grandes en Rosario, para una segunda etapa:**

- **La Reina, Arco Iris, Alfa:** cadenas locales medianas que aparecen en el padrón provincial.
- **Makro:** mayorista con una sucursal en Rosario, [ahora de Cencosud](https://www.lacapital.com.ar/economia/un-gigante-extranjero-compro-los-supermercados-makro-argentina-n10177073.html).

## Fuentes de datos por cadena

Primera revisión el 10/11/2026; el 11/11/2026 se verificaron las tres fuentes del MVP con requests reales. Resumen: **Coto y Carrefour tienen precios de Rosario** pasándole a su API el código de tienda o región correcto. DAR se lee del HTML. La Anónima publica folletos por ciudad con texto extraíble. Jumbo online no cubre Rosario, Todo Chen no tiene web y La Gallega bloquea la lectura automática. El detalle técnico de cada endpoint está en `docs/context.md`.

| Cadena | Web | Cómo se lee | Qué trae | ¿Precio de Rosario? | Dificultad |
| --- | --- | --- | --- | --- | --- |
| DAR | [darentucasa.com.ar](https://www.darentucasa.com.ar/) | HTML server-side (ASP). Se abre la sesión, se manda el POST de Ofertas (`TM=CM`) y se paginan `productos.asp?page=N` | Ofertas con precio final (sin tachado). ~300 ofertas en 15 páginas. Promos bancarias en banners | Sí, despacha desde (dirección quitada) | Media: hay que mantener la sesión y mandar headers HTML normales, sin navegador headless |
| Coto | [coto.com.ar](https://www.coto.com.ar/productos/ofertas/todas-las-ofertas) | Endpoint Endeca: `/sitios/cdigi/categoria/_/N-<tipo de oferta>?format=json`, filtrando disponibilidad en la tienda 165. La API nueva (`api.coto.com.ar`) pide una key y se descartó | EAN, precio de lista, precio con descuento, texto de la promo, descuentos por medio de pago y disponibilidad por tienda. ~3.000 ofertas solo en "Hasta 30% DTO" para Alto Rosario | **Parcial:** el precio es el de la lista online (200); el filtro de la 165 deja solo lo disponible en Alto Rosario. El brief original vio que la 165 coincidía con la 200 | Baja. Se pagina de a 50 por tipo de oferta |
| Carrefour | [carrefour.com.ar](https://www.carrefour.com.ar/) | API VTEX Intelligent Search: `/api/io/_v/api/intelligent-search/product_search/category-1/<categoría>` con `regionId` = base64 de `SW#carrefourar0018`, ordenado por descuento y sin productos agotados | `Price`, `ListPrice`, EAN y stock, por región | **Sí.** Con región de Rosario los precios cambian: yerba Mañanita 1 kg a $4.589 contra $5.749 sin región (verificado de nuevo el 11/11) | Baja. La API corta en ~2.500 resultados: se consulta por categoría |
| Jumbo | [jumbo.com.ar](https://www.jumbo.com.ar/) | Misma API VTEX | — | **No**: sin puntos de retiro cerca de Rosario ni región para CP 2000. Jumbo Portal Rosario sale de SEPA | — |
| La Anónima | [catálogos](https://www.laanonima.com.ar/empresa/catalogos) (Santa Fe → Rosario) | Folletos en visor *flipbook*: cada uno tiene `spreads.json` con el texto de cada página, y un PDF descargable | Folletos de zona `z02`: Frescos, Canasta Ahorro, Insuperables y especiales. Ej.: Insuperables del 05/11 al 15/11, 36 páginas | Sí, folletos por ciudad | Media: el texto viene desordenado |
| La Gallega | [lagallega.com.ar](https://www.lagallega.com.ar/) | - | - | - | Fuera de alcance: su `robots.txt` (`Disallow: /*?`) bloquea todas las URLs con parámetros, incluidas las de ofertas |
| Todo Chen | Sin web; solo Instagram | — | — | — | Fuera de alcance: scrapear Instagram va contra sus términos y se rompe seguido |

**Detalles que importan para el agente:**

- **Cómo encontrar la región de Carrefour:** `/api/checkout/pub/pickup-points?geoCoordinates=-60.65;-32.95` devuelve las sucursales cercanas y su *seller*. Para Rosario: `carrefourar0018` (Fisherton, también retiro en Rosario Sur) y `carrefourar0268` (Pueyrredón). Los `carrefourar0899` son de Hogar & Electro, no de súper.
- **Coto, a verificar:** si las cuatro sucursales chicas de Rosario usan la misma lista que la 165. En las 150 ofertas que comparé, la lista 165 coincidió con la online (200), pero otras zonas sí difieren (la lista 060 tenía una leche a $1.390 contra $2.499).
- **Coto, cuidado con los campos:** el precio normal es `precioLista` y el de oferta es `precioDescuento`. El campo `precio` de `sku.dtoPrice` no es el precio del producto.
- **El precio tachado no siempre es confiable.** En Jumbo, una yerba de $8.900 venía con `ListPrice` de $735.537. Hay que validar el "precio anterior" antes de calcular un %.
- **DAR no muestra precio anterior.** El descuento real de DAR sale del historial propio.
- **Todo es público y sin login** para ver ofertas. Igual: pocas requests, User-Agent identificable (nunca hacerse pasar por un navegador), respetar `robots.txt` y no usar keys sacadas de las páginas.

## ¿Conviene bajar el folleto y scrapearlo?

Sí, pero solo donde no hay algo mejor: hoy aplica a **La Anónima**, y sirve como complemento para ofertas exclusivas de sucursal que no aparecen en la web. Cuando una cadena tiene API o HTML con precios, el folleto es más trabajo y menos preciso.

Orden de preferencia por fuente, de más confiable a menos:

1. **API / JSON** (Coto, Carrefour): estructurado, con precio por región.
2. **HTML del sitio** (DAR): estable mientras no rediseñen.
3. **Texto embebido en el folleto** (La Anónima): el visor ya trae el texto de cada página en `spreads.json`, o se saca de la capa de texto del PDF. No hace falta OCR.
4. **OCR sobre la imagen del folleto**: último recurso; confunde centavos, precios por kg y precios sin impuestos.

Para el folleto de La Anónima, el problema no es leerlo sino armar los pares. El texto de una página mezcla precio final, precio por kg/litro, "precio sin impuestos nacionales" y el nombre del producto en renglones separados. Dos caminos:

- **Reglas por layout:** rápidas y gratis, pero se rompen cuando cambia el diseño del folleto.
- **Un LLM que extraiga** `{producto, precio, precio_anterior, vigencia}` por página, validado después contra el historial. Más robusto; tiene costo por llamada.

Los folletos son semanales y traen la vigencia impresa (ej.: del 05/11 al 15/11), así que alcanza con procesarlos una vez por semana, no todos los días.

## Fuente oficial: SEPA / Precios Claros

Es la única fuente que da precio por cada sucursal física, la única forma de cubrir Jumbo Portal Rosario, y sirve para validar Coto y Carrefour. Las cadenas [tienen que cargar sus precios todos los días a las 6 de la mañana](https://www.iprofesional.com/economia/395412-precios-el-gobierno-lanza-una-app-para-comprar-mas-barato), y el dataset se publica como datos abiertos en [datos.produccion.gob.ar/dataset/sepa-precios](https://www.datos.gob.ar/th/dataset/produccion-precios-claros---base-sepa), con un archivo por día de la semana.

- **Tamaño:** el paquete diario ronda los [4 GB](https://www.lavozdesanjusto.com.ar/el-gobierno-anuncio-la-puesta-en-marcha-de-una-plataforma-de-informacion-de-precios-diarios). Hay que bajarlo, filtrar solo las sucursales de Rosario y descartar el resto en el mismo paso, en streaming (Rust lo hace sin cargar todo en memoria).
- **Referencia de código:** el [scraper de OpenDataCórdoba](https://github.com/OpenDataCordoba/precios_claros) ya baja productos, precios y sucursales del portal, y [preciazo](https://github.com/catdevnull/preciazo) scrapea supermercados argentinos a SQLite.
- **A verificar:** si DAR reporta a SEPA (que aparezca en el padrón provincial "Precios Santafesinos" es otro programa), y si el CSV trae columnas de precio promocional o solo precio de lista.

No pude abrir la página del dataset desde el navegador (permiso del sitio), así que la estructura exacta de columnas queda para que el agente la revise.

## Qué cuenta como descuento

Con miles de ofertas por cadena (≈ 300 en DAR, miles en Coto y Carrefour), mostrar *todo* en la portada es inservible. La regla propuesta: se guarda y se publica todo en los datos, pero la portada destaca solo lo que pasa un filtro.

| Tipo | De dónde sale | Cómo se calcula | ¿Se destaca en la portada? |
| --- | --- | --- | --- |
| Precio tachado | Precio anterior vs. precio oferta en la web | % = 1 − oferta / anterior | Solo si supera un umbral (ej.: 25%) y el precio anterior es razonable |
| Baja real | Historial propio en la base | Precio de hoy vs. mediana de los últimos 30 días | Sí: es la señal más honesta |
| Promo por cantidad | Etiquetas tipo 2x1, 3x2, 70% en la 2da | Precio efectivo por unidad | Sí, con el precio por unidad calculado |
| Bancaria | Páginas de beneficios (DAR) y descuentos por medio de pago (Coto) | Banco, día, %, tope | En una página aparte por día de la semana, no por producto |

**Reglas de sanidad** antes de destacar: descartar precios anteriores más de 5 veces mayores que el actual (el caso de Jumbo), productos sin stock y descuentos que se repiten igual todas las semanas.

## Arquitectura

Todo corre en una sola ejecución diaria de GitHub Actions: no hace falta servidor, el costo es cero y el historial de precios queda versionado en el mismo repo ("git scraping").

1. **Scrapers** (uno por cadena, aislados) → lista de `Offer`.
2. **Detector** → reglas de descuento y de sanidad.
3. **Generador del sitio** → HTML estático por página, JSON/CSV, `sitemap.xml`, `llms.txt`.
4. **Deploy** → `wrangler deploy` a Cloudflare Workers (static assets) con un token de API. Cloudflare no se conecta al repo.
5. **Commit** de `data/` con las ofertas del día.

**Stack:** Rust para scrapers, detector y generación del HTML; JS plano sin framework para búsqueda, filtros, favoritos e instalación; PWA con `manifest.webmanifest` + `sw.js`. Detalle en `docs/context.md`.

Estructura de repo (código en inglés, todo lo demás en español):

```text
ofertas-rosario/
├── Cargo.toml
├── config/
│   └── stores.toml          # cadenas y sucursales de Rosario, editables por PR
├── src/
│   ├── main.rs              # CLI: scrape, build
│   ├── model.rs             # Offer: chain, store, product, ean, price, list_price, promo, source
│   ├── http.rs              # cliente con User-Agent, pausas y reintentos
│   ├── scrapers/
│   │   ├── dar.rs
│   │   ├── coto.rs
│   │   └── carrefour.rs
│   ├── detector.rs          # reglas de descuento y de sanidad
│   └── site.rs              # genera HTML, JSON/CSV, sitemap, llms.txt
├── templates/               # plantillas HTML
├── web/                     # sw.js, manifest, app.js, íconos
├── data/                    # JSON diario por cadena (historial)
├── tests/                   # falla si una cadena trae muchas menos ofertas que lo habitual
├── wrangler.jsonc
└── .github/workflows/daily.yml
```

- **Un contrato común (`Offer`)** con un campo `source` (web o SEPA), así el detector no sabe ni le importa de qué cadena viene el dato.
- **Las sucursales en un archivo de configuración**, no en código: cualquiera puede corregir una dirección con un PR sin saber programar.
- **Una cadena que falla no frena a las demás:** cada scraper corre aislado y su error queda en el log y en un issue.
- **SEO y descubrimiento por IAs:** el HTML llega con las ofertas escritas (no es una SPA vacía), con JSON-LD, sitemap, `robots.txt` abierto a buscadores y bots de IA, `llms.txt` y datos abiertos.

## MVP, roadmap y riesgos

El MVP arranca por las tres fuentes con precio de Rosario y lectura directa: DAR, Coto (tienda 165) y Carrefour (región de Rosario). Si funcionan dos semanas seguidas sin intervención, se suman los folletos y SEPA.

1. **MVP:** scrapers de DAR, Coto y Carrefour → JSON diario en el repo → web estática instalable (PWA) con las mejores ofertas del día, una página por cadena y datos abiertos.
2. **Historial:** guardar precios día a día para detectar bajas reales y precios tachados inflados.
3. **Folletos:** La Anónima, una vez por semana.
4. **SEPA:** Jumbo Portal Rosario y validación cruzada de las demás.
5. **Lista de seguimiento:** productos favoritos guardados en el celu (sin cuentas).
6. **Notificaciones push** de la propia PWA, con un Worker chico que guarde las suscripciones.
7. **Otras cadenas:** Todo Chen si publica en algún lado scrapeable, y cadenas locales medianas.

| Riesgo | Mitigación |
| --- | --- |
| Una web cambia y el scraper devuelve 0 productos sin avisar | Test que falla si una cadena trae menos del 50% de lo habitual; abre un issue automático |
| Bloqueo por anti-bot (más probable en Coto) o de las IPs de GitHub Actions | Pocas requests, una corrida diaria, y SEPA como respaldo. Probar desde Actions antes de dar por cerrada cada fuente |
| Precio online distinto al de góndola | Etiquetar cada dato con su fuente (web vs. SEPA) y avisarlo en la web |
| Ruido en la portada | Umbral alto + historial + top N diario |
| Términos de uso de cada sitio | Revisarlos antes de publicar; uso no comercial y datos públicos. Respetar `robots.txt` (por eso se sacó La Gallega) |

## Preguntas abiertas para el agente

- [x] ~~¿La Gallega y DAR comparten plataforma?~~ La Gallega quedó afuera, ya no aplica.
- [x] ¿Cómo se arma el POST de ofertas de DAR? Se emula sin navegador: sesión + `POST carrito.asp` con `TM=CM` + `productos.asp?page=N` (ver `docs/context.md`).
- [ ] ¿DAR aparece en SEPA? ¿SEPA trae columnas de promo?
- [ ] ¿Las sucursales chicas de Coto en Rosario usan la lista de precios de la 165? ¿Y Carrefour Pellegrini y Village tienen seller propio?
- [ ] ¿El paquete diario de SEPA se puede procesar en GitHub Actions o hace falta otro runner?
- [x] Lenguaje del proyecto: Rust para el pipeline, JS plano en el navegador.
- [ ] Licencia (MIT para el código; los datos, aclarar que son de fuentes públicas).
- [ ] Nombre del proyecto y dominio.

## Fuentes

Webs visitadas el 10/11/2026: [La Gallega](https://www.lagallega.com.ar/), [DAR en tu casa](https://www.darentucasa.com.ar/), [Coto sucursales](https://www.coto.com.ar/sucursales/), [Coto ofertas](https://www.coto.com.ar/productos/ofertas/todas-las-ofertas), [Carrefour](https://www.carrefour.com.ar/), [Jumbo](https://www.jumbo.com.ar/). Sucursales: [padrón Precios Santafesinos](https://www.santafe.gov.ar/precios-santafesinos/listadocomercios). Mercado rosarino: [Conclusión](https://www.conclusion.com.ar/?p=307450), [Letra P](https://www.letrap.com.ar/economia/la-familia-braun-logra-hacer-pie-rosario-y-abre-su-primera-sucursal-la-anonima-n5424156). Referencia de otro proyecto: [ratoneando-go](https://github.com/matiasbontempo/ratoneando-go) (endpoint Endeca de Coto).
