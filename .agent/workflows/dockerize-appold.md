---
description: Dockerファイル、docker-compose.yml 生成
---

---
description: Dockerize the current FastAPI/Uvicorn Python app with best practices including BuildKit cache mount, curl for healthcheck, persistent data volumes, log rotation, non-root user, and production-ready optimizations
---

You are an expert DevOps engineer specializing in Docker + FastAPI production deployments.

Task: Containerize the current project as a production-ready FastAPI application using Uvicorn.

Follow these strict rules and best practices (2025 standards):

1. Use multi-stage build if it makes sense (but for most FastAPI apps, single stage with cache mount is fine and simpler)
2. Base image: python:3.12-slim (or 3.13-slim if project supports)
3. Always start with syntax directive for BuildKit cache:
   # syntax=docker/dockerfile:1

4. Use --mount=type=cache,target=/root/.cache/pip for pip install to cache downloads & wheels → super fast rebuilds!
5. Install curl for healthcheck (minimal, --no-install-recommends)
6. Create necessary directories for persistent data (chroma, huggingface, logs, etc.)
7. Copy code only after dependencies (cache layer optimization)
8. Set environment variables for paths (CHROMA_PATH, HF_HOME, PYTHONUNBUFFERED=1)
9. Run as non-root user for security (addgroup + adduser)
10. Expose the port dynamically via ENV APP_PORT
11. CMD uses uvicorn with --host 0.0.0.0 --port ${APP_PORT:-8000}
12. Add HEALTHCHECK using curl to /docs or /health if exists (prefer /docs for FastAPI)
13. Generate docker-compose.yml with:
    - build: .
    - network_mode: host (if Ollama etc. needs localhost)
    - volumes for ./datas and ./logs
    - environment variables
    - logging: json-file with max-size 10m, max-file 3
    - healthcheck same as Dockerfile
    - restart: unless-stopped

Steps to follow:
1. Analyze the current project structure (src/main.py assumed for FastAPI app)
2. Check if requirements.txt exists → if not, generate it
3. Create Dockerfile in project root
4. Create docker-compose.yml in project root
5. Add .dockerignore if missing (ignore __pycache__, .git, datas/* except structure, etc.)
6. Output both files as artifacts
7. Explain changes and why they improve build speed/security/reliability
8. If user wants Gunicorn for production workers → ask, but default is single Uvicorn (good for most cases)

Be thorough, production-grade, and add comments in the files.

Now, dockerize the current workspace!