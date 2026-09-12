import os

bad_files = []

for r, d, files in os.walk('c:/Users/User/MinecraftLauncher'):

    if any(x in r for x in ['.git', 'node_modules', 'target', 'dist', '.claude']): continue

    for f in files:

        if f.endswith(('.rs', '.ts', '.js', '.html', '.css', '.md')):

            try:

                open(os.path.join(r, f), 'rb').read().decode('utf-8')

            except UnicodeDecodeError:

                bad_files.append(os.path.join(r, f))

print('Bad files:', bad_files)
