# Cómo colaborar

Gracias por querer sumar. Ofertas Rosario es un proyecto chico, sin fines de lucro, y cualquier ayuda sirve: avisar de un precio mal leído, corregir una sucursal, mejorar la web o sumar una cadena.

Antes de arrancar, leé [`STATUS.md`](STATUS.md) (qué está hecho, qué falta y qué se decidió) y [`docs/context.md`](docs/context.md) (cómo se lee cada supermercado).

## Sin programar

- **Avisar de un error:** abrí un issue con la oferta (cadena, producto, precio que muestra la web y precio real) y, si podés, una captura. Si no tenés cuenta de GitHub, escribí al mail que figura en [Preguntas frecuentes](https://ofertas-rosario.rindelloezequiel.workers.dev/preguntas-frecuentes/).
- **Corregir una sucursal o un texto de ayuda:** están en [`config/stores.toml`](config/stores.toml). Se puede editar desde la web de GitHub (el lápiz arriba a la derecha) y mandar el cambio como pull request.
- **Proponer una cadena:** abrí un issue con el nombre, la web y si tiene sucursales en Rosario. Antes de sumarla hay que revisar su `robots.txt` y sus términos de uso (ver más abajo).

## Correrlo en tu compu

Requisito: Rust estable ([rustup](https://rustup.rs)). En Windows conviene WSL: es el mismo Linux que usa GitHub Actions y no pide las herramientas de C++ de Visual Studio.

```bash
cargo test                                     # tests con respuestas guardadas, sin red
cargo run --release -- scrape --only dar       # leer una cadena (escribe data/)
cargo run --release -- build                   # generar el sitio en dist/ a partir de data/
python -m http.server 8787 --directory dist    # verlo en http://127.0.0.1:8787
```

Para trabajar en la web no hace falta scrapear: `build` usa los datos que ya están en `data/`. Si scrapeaste para probar, no incluyas `data/` en el pull request: esa carpeta la actualiza un bot todos los días.

## Reglas de cortesía con los sitios

El proyecto lee webs públicas de empresas que no lo pidieron. Para que pueda seguir haciéndolo, cualquier cambio en los scrapers tiene que respetar esto:

- **User-Agent identificable**, el de `config/stores.toml`. Nunca hacerse pasar por un navegador.
- **Respetar `robots.txt`.** Si un sitio bloquea sus páginas de ofertas, no se lee (por eso La Gallega no está).
- **Una corrida por día**, con pausa entre requests y sin paralelismo agresivo.
- **Nada de keys ni tokens sacados de las páginas** para usar APIs privadas. Solo lo que el sitio sirve en público.
- **Si una cadena pide que dejemos de mostrar sus datos, se saca.**

## Sumar una cadena

1. Revisá su `robots.txt` y sus términos de uso, y anotá en el issue de dónde salen los precios (API pública, HTML, etc.) y para qué sucursal de Rosario son.
2. Sumá la variante en `Chain` (`src/model.rs`) y su sección en `config/stores.toml` y `src/config.rs`.
3. Escribí el scraper en `src/scrapers/<cadena>.rs` y registralo en `src/scrapers/mod.rs` y en `src/main.rs`.
4. Guardá una respuesta real en `tests/fixtures/` y testeá el parseo contra ese archivo, como hacen los scrapers de Carrefour, Coto y DAR. Los tests no usan la red.
5. Documentá la fuente en `docs/context.md`: endpoint, parámetros, campos que se usan y cualquier cosa rara que hayas encontrado.

## Pull requests

- Todo en español (docs, textos de la web, issues y PRs), menos el código: identificadores y comentarios en inglés.
- Mensajes de commit con la forma `tipo: descripción`, en español (por ejemplo `fix: precio de Coto sin descuento`).
- `cargo test` tiene que pasar.
- Si cambiás la web, probala en el celu o con un ancho de 390 px, en modo claro y oscuro. La paleta y la tipografía están en `web/styles.css`: no sumes colores, fuentes ni recursos externos sin charlarlo antes en un issue.
- Si cambia cómo se lee una fuente, actualizá `docs/context.md`.

## Licencia

Al mandar un cambio aceptás que se publique con la licencia [MIT](LICENSE) del proyecto.
