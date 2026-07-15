#!/bin/bash

# =============================================================================
# AppFlowy Cloud - GoTrue Mailer Support Email Setter
# =============================================================================
# The GoTrue auth email templates (confirmation, recovery, magic link, ...)
# under assets/mailer_templates/*.html are static files fetched by GoTrue at
# runtime from a URL (see GOTRUE_MAILER_TEMPLATES_* in dev.env/deploy.env).
# GoTrue does not render them through this app, so the support email address
# baked into these files can't be swapped at runtime via an env var like the
# AppFlowy-Cloud mailer templates (APPFLOWY_MAILER_SUPPORT_EMAIL) can.
#
# Run this script once to replace the default support email in those static
# templates, then commit the result and point your GOTRUE_MAILER_TEMPLATES_*
# env vars at the branch/fork containing your customized copies.
#
# Usage: ./script/set_mailer_support_email.sh support@yourdomain.com
# =============================================================================

set -e

PROJECT_ROOT="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
TEMPLATES_DIR="$PROJECT_ROOT/assets/mailer_templates"
DEFAULT_EMAIL="support@appflowy.io"

NEW_EMAIL="$1"
if [ -z "$NEW_EMAIL" ]; then
  echo "Usage: $0 <new-support-email>"
  exit 1
fi

FOUND=0
for file in "$TEMPLATES_DIR"/*.html; do
  [ -f "$file" ] || continue
  if grep -q "$DEFAULT_EMAIL" "$file"; then
    sed -i.bak "s/$DEFAULT_EMAIL/$NEW_EMAIL/g" "$file"
    rm -f "$file.bak"
    echo "Updated $(basename "$file")"
    FOUND=1
  fi
done

if [ "$FOUND" -eq 0 ]; then
  echo "No occurrences of $DEFAULT_EMAIL found under $TEMPLATES_DIR"
  exit 1
fi

echo ""
echo "Done. Review the changes, commit them, and point your"
echo "GOTRUE_MAILER_TEMPLATES_* env vars at the branch/fork containing them."
