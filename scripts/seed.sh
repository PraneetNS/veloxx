#!/usr/bin/env bash
set -e

# Seeding Veloxx with default tenant and admin user.

DATABASE_URL="postgres://veloxx:password@localhost:5432/veloxx"

echo "Seeding default data..."
psql "$DATABASE_URL" -f scripts/seed_data.sql

echo "Seed data inserted!"
echo "Sign in at http://localhost:3000/login"
echo "Email: admin@veloxx.ai"
echo "Password: admin123"
