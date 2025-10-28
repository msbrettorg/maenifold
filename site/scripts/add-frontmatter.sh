#!/bin/bash
# Add frontmatter to tool markdown files

TOOLS_DIR="/Users/brett/src/ma-collective/maenifold/site/src/pages/usage/tools"

cd "$TOOLS_DIR" || exit 1

for file in *.md; do
    if [ -f "$file" ]; then
        # Check if file already has frontmatter
        if head -1 "$file" | grep -q "^---$"; then
            echo "Skipping $file (already has frontmatter)"
        else
            echo "Adding frontmatter to $file"
            # Create temp file with frontmatter
            {
                echo "---"
                echo "layout: ../../../layouts/DocsLayout.astro"
                echo "---"
                echo ""
                cat "$file"
            } > "$file.tmp"
            mv "$file.tmp" "$file"
        fi
    fi
done

echo "Done!"
