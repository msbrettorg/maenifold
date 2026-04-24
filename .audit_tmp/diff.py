import json, re, os

def normalize(s):
    if not s: return ''
    # strip whitespace and comments
    s = re.sub(r'//.*', '', s)
    s = re.sub(r'/\*.*?\*/', '', s, flags=re.DOTALL)
    return re.sub(r'\s+', ' ', s).strip()

def prop_names(blocks):
    """Extract prop names from blocks of either defineProps<{...}>(...) form or defineProps({...})"""
    names = set()
    for b in blocks:
        if not b: continue
        # generic form
        if b.startswith('GENERIC:'):
            body = b[len('GENERIC:'):]
            # Match identifier: like "name?: type" or "name: type"
            for m in re.finditer(r'(?:^|[{,;])\s*([a-zA-Z_$][\w$]*)\s*\??\s*:', body):
                names.add(m.group(1))
        else:
            # withDefaults wrapper or object literal
            # try TS object
            for m in re.finditer(r'(?:^|[{,])\s*([a-zA-Z_$][\w$]*)\s*\??\s*:', b):
                names.add(m.group(1))
            # array form: defineProps(['a', 'b'])
            for m in re.finditer(r"['\"]([a-zA-Z_$][\w$]*)['\"]", b):
                names.add(m.group(1))
    # Filter out keywords/values
    junk = {'type', 'default', 'required', 'validator', 'String', 'Number', 'Boolean', 'Object', 'Array', 'Function'}
    return names - junk

def emit_names(blocks):
    names = set()
    for b in blocks:
        if not b: continue
        if b.startswith('GENERIC:'):
            body = b[len('GENERIC:'):]
            # (e: 'event-name', ...) form
            for m in re.finditer(r"['\"]([\w:-]+)['\"]", body):
                names.add(m.group(1))
        else:
            for m in re.finditer(r"['\"]([\w:-]+)['\"]", b):
                names.add(m.group(1))
    return names

def model_names(blocks):
    names = set()
    for b in blocks:
        if not b: continue
        # defineModel('name', ...) or defineModel<T>() (default model)
        m = re.search(r"['\"]([\w-]+)['\"]", b)
        if m:
            names.add(m.group(1))
        else:
            names.add('(default)')
    return names

with open('.audit_tmp/summary.json') as f:
    data = json.load(f)

issues = []
for d in data:
    op = prop_names(d['old_props'])
    np = prop_names(d['new_props'])
    oe = emit_names(d['old_emits'])
    ne = emit_names(d['new_emits'])
    om = model_names(d['old_models'])
    nm = model_names(d['new_models'])
    
    missing_props = op - np
    new_props = np - op
    missing_emits = oe - ne
    new_emits = ne - oe
    missing_models = om - nm
    new_models = nm - om
    
    if missing_props or new_props or missing_emits or new_emits or missing_models or new_models:
        issues.append({
            'path': d['path'],
            'missing_props': sorted(missing_props),
            'new_props': sorted(new_props),
            'missing_emits': sorted(missing_emits),
            'new_emits': sorted(new_emits),
            'missing_models': sorted(missing_models),
            'new_models': sorted(new_models),
        })

print(f"Total files: {len(data)}")
print(f"Files with interface differences: {len(issues)}")
print()
for i in issues:
    print(f"=== {i['path']} ===")
    if i['missing_props']: print(f"  MISSING props: {i['missing_props']}")
    if i['new_props']: print(f"  ADDED props: {i['new_props']}")
    if i['missing_emits']: print(f"  MISSING emits: {i['missing_emits']}")
    if i['new_emits']: print(f"  ADDED emits: {i['new_emits']}")
    if i['missing_models']: print(f"  MISSING models: {i['missing_models']}")
    if i['new_models']: print(f"  ADDED models: {i['new_models']}")

with open('.audit_tmp/issues.json', 'w') as f:
    json.dump(issues, f, indent=2)
