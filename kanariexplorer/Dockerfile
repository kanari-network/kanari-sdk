# Use bun as the base image
FROM oven/bun:1.1 AS base
WORKDIR /app

# Install dependencies
FROM base AS install
COPY package.json ./
# Remove any existing lockfile that might be incompatible and let bun generate a new one or just install
RUN bun install

# Build the application
FROM base AS build
COPY --from=install /app/node_modules ./node_modules
COPY . .
# Set environment variables for build
ENV NEXT_TELEMETRY_DISABLED=1
RUN bun run build

# Runtime stage
FROM base AS runner
WORKDIR /app

ENV NODE_ENV=production
ENV NEXT_TELEMETRY_DISABLED=1

# Copy built files
COPY --from=build /app/public ./public
COPY --from=build /app/.next ./.next
COPY --from=build /app/node_modules ./node_modules
COPY --from=build /app/package.json ./package.json

# Expose port
EXPOSE 3000

# Start the application
CMD ["bun", "run", "start"]
