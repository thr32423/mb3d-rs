import re,glob,json,sys
calls=r'(?:Text|Button|Toggle|Label|Section|LabeledContent|Picker|Menu|Stepper|ColorPicker|ContentUnavailableView|navigationTitle|page|alert|ShareLink|TextField|accessibilityLabel|String\(localized:)'
lit=r'"((?:[^"\\]|\\.|\\\((?:[^()"]|"[^"]*"|\([^()]*\))*\))*)"'
pat=re.compile(r'(?:\b'+calls+r'\(\s*|title:\s*\(?\[?|\?\s*|:\s*)'+lit)
# the interpolations of the app's texts: whole numbers %lld, the rest %@
ints={'index','number','s.id + 1','key.frames','sculptor.smoothing','breeder.generation','i + 1','n','done.count',
      'studio.doc.formulas.firstIndex { $0.id == section.id }.map { $0 + 1 } ?? 1'}
def key(s):
    out=[];i=0
    while i<len(s):
        if s.startswith('\\(',i):
            d=0;j=i+1
            while True:
                if s[j]=='(':d+=1
                elif s[j]==')':
                    d-=1
                    if d==0:break
                j+=1
            e=s[i+2:j].strip()
            out.append('%lld' if e in ints else '%@');i=j+1
        else:
            if s.startswith('\\"',i): out.append('"'); i+=2; continue
            out.append(s[i]);i+=1
    return ''.join(out)
keys={}
skip=re.compile(r'^(#|[a-z_0-9]+$|[a-z]+\.[a-z.]+$|%|\.|-)')
for p in sorted(glob.glob(sys.argv[1]+'/*.swift')):
    src=open(p).read()
    for m in pat.finditer(src):
        s=m.group(1)
        try: k=key(s)
        except IndexError: print("skipped", s[:60], file=sys.stderr); continue
        if not re.search('[A-Za-z]{2}',k) or skip.match(k): continue
        keys.setdefault(k,p.split('/')[-1])
for k in ["Turn about x","Turn about y","Turn about z"]: keys.setdefault(k,'Settings.swift')
json.dump(sorted(keys), open(sys.argv[2],'w'), ensure_ascii=False, indent=1)
print(len(keys))
