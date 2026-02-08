# Use bun as the base image
FROM oven/bun:latest AS base
WORKDIR /app

# Install dependencies
FROM base AS install
COPY package.json ./
RUN bun install

# Build the application
FROM base AS build
COPY --from=install /app/node_modules ./node_modules
COPY . .
ENV NEXT_TELEMETRY_DISABLED=1
RUN bun run build

# Runtime stage
# Use a very small image for the runner
FROM node:20-slim AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

# Copy standalone build and static files
# Next.js standalone mode includes only the necessary files
COPY --from=build /app/.next/standalone ./
COPY --from=build /app/.next/static ./.next/static
COPY --from=build /app/public ./public

# Expose port
EXPOSE 3000

# Next.js standalone uses server.js
CMD ["node", "server.js"]
