import re, sys, subprocess, os, json

def get_old(path):
    r = subprocess.run(['git', 'show', f'HEAD:{path}'], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ''

def get_new(path):
    try:
        with open(path) as f: return f.read()
    except: return ''

def extract_script(text):
    m = re.search(r'<script[^>]*>(.*?)</script>', text, re.DOTALL)
    return m.group(1) if m else ''

def extract_template(text):
    m = re.search(r'<template[^>]*>(.*)</template>', text, re.DOTALL)
    return m.group(1) if m else ''

def find_block(text, fn_name):
    """Find defineProps/defineEmits/defineModel call and return the args text."""
    results = []
    for m in re.finditer(rf'\b{fn_name}\s*(?:<[^>]*>)?\s*\(', text):
        start = m.end()
        depth = 1
        i = start
        while i < len(text) and depth > 0:
            c = text[i]
            if c == '(': depth += 1
            elif c == ')': depth -= 1
            i += 1
        results.append(text[start:i-1])
    # Also catch generic style: defineProps<T>() with no args
    for m in re.finditer(rf'\b{fn_name}\s*<([^>]+)>\s*\(\s*\)', text):
        results.append('GENERIC:' + m.group(1))
    return results

def extract_props(script):
    blocks = find_block(script, 'defineProps')
    return blocks

def extract_emits(script):
    return find_block(script, 'defineEmits')

def extract_models(script):
    return find_block(script, 'defineModel')

def summary(path):
    old = get_old(path)
    new = get_new(path)
    if not old or not new:
        return None
    olds, news = extract_script(old), extract_script(new)
    return {
        'path': path,
        'old_props': extract_props(olds),
        'new_props': extract_props(news),
        'old_emits': extract_emits(olds),
        'new_emits': extract_emits(news),
        'old_models': extract_models(olds),
        'new_models': extract_models(news),
    }

with open('.audit_tmp/files.txt') as f:
    files = [l.strip() for l in f if l.strip()]

results = []
for p in files:
    s = summary(p)
    if s: results.append(s)

with open('.audit_tmp/summary.json', 'w') as f:
    json.dump(results, f, indent=2)
print(f"Wrote {len(results)} summaries")
