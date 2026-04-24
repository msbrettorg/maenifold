import re, subprocess, json

def get_old(path):
    r = subprocess.run(['git', 'show', f'HEAD:{path}'], capture_output=True, text=True)
    return r.stdout if r.returncode == 0 else ''

def get_new(path):
    try:
        with open(path) as f: return f.read()
    except: return ''

def script(text):
    m = re.search(r'<script[^>]*>(.*?)</script>', text, re.DOTALL)
    return m.group(1) if m else ''

# Strip comments + collapse whitespace for comparison
def normalize(s):
    s = re.sub(r'//[^\n]*', '', s)
    s = re.sub(r'/\*.*?\*/', '', s, flags=re.DOTALL)
    s = re.sub(r'\s+', ' ', s).strip()
    return s

with open('.audit_tmp/files.txt') as f:
    files = [l.strip() for l in f if l.strip()]

changed_script = []
identical_script = []
for p in files:
    o = normalize(script(get_old(p)))
    n = normalize(script(get_new(p)))
    if o == n:
        identical_script.append(p)
    else:
        changed_script.append((p, len(o), len(n)))

print(f"Files with IDENTICAL script blocks: {len(identical_script)}")
print(f"Files with CHANGED script blocks: {len(changed_script)}")
print()
print("Changed scripts:")
for p, ol, nl in changed_script:
    print(f"  {p}  (old={ol}ch, new={nl}ch, diff={nl-ol:+d})")
