# ALICE-Render-SaaS

Software rendering as a Service — physically based frame rendering, scene management, material authoring, and preset library via REST API.

## Architecture

```
Client
  |
  v
API Gateway (:8220)
  |
  v
Core Engine (:8120)
  |
  +-- Ray Tracer / Path Tracer
  +-- Scene Manager
  +-- Material Compiler
  +-- Preset Registry
```

## Features

- Path-traced physically based rendering (PBR)
- HDR environment lighting (EXR, latlong)
- Principled BSDF material model
- Denoising (temporal accumulation + blue noise)
- Render preset library (studio, outdoor, product, cinematic)
- Tile-based distributed rendering support

## API Endpoints

### Core Engine (port 8120)

| Method | Path | Description |
|--------|------|-------------|
| POST | /api/v1/render/frame | Render a single frame from a scene description |
| POST | /api/v1/render/scene | Upload or update a scene graph |
| POST | /api/v1/render/material | Compile and register a PBR material |
| GET  | /api/v1/render/presets | List available render presets |
| GET  | /api/v1/render/stats | Return runtime statistics |
| GET  | /health | Health check |

### Example: Render Frame

```bash
curl -X POST http://localhost:8120/api/v1/render/frame \
  -H 'Content-Type: application/json' \
  -d '{"scene_id":"sc1","width":1920,"height":1080,"samples":256,"preset":"studio"}'
```

### Example: Register Material

```bash
curl -X POST http://localhost:8120/api/v1/render/material \
  -H 'Content-Type: application/json' \
  -d '{"name":"chrome","base_color":[0.8,0.8,0.8],"metallic":1.0,"roughness":0.05}'
```

## License

AGPL-3.0-or-later
