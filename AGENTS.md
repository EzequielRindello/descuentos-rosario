# Ofertas Rosario: instrucciones para agentes

Antes de hacer cualquier cosa, leé `STATUS.md` (estado, decisiones, tareas) y `docs/context.md` (referencia técnica de cada fuente).

- Mantené `STATUS.md` al día con cada cambio que importe: marcá tareas, sumá decisiones nuevas con fecha y motivo.
- Los commits los hace el mantenedor a mano. El agente propone el mensaje (`tipo: descripción`, en español) y, cuando le pasan el hash, lo carga en la tabla de commits de `STATUS.md`.
- Si cambia cómo se lee una fuente, actualizá `docs/context.md`.
- Todo en español (docs, web, README), menos el código (identificadores y comentarios en inglés).
- Nada de estilos, fuentes ni colores hasta que se pida explícitamente.
- Comandos para compilar, testear y scrapear: sección "Cómo correrlo" de `STATUS.md`.
- Un bot commitea `data/` en `main` todos los días: hacé `git pull` antes de empezar. No edites `data/` a mano.
- Publicar (`wrangler deploy`), disparar el workflow o tocar secrets de GitHub/Cloudflare: pedir OK antes.
- Cortesía con los sitios: User-Agent identificable, respetar `robots.txt`, nunca hacerse pasar por un navegador ni usar keys sacadas de las páginas.
- Sin rutas locales ni datos personales en el repo: hoy es privado, pero se hace público al lanzar el MVP.
