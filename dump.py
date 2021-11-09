#!/usr/bin/env nix-shell
#!nix-shell -i python3 -p python3 python3Packages.toml python3Packages.pillow
import musicpd
import re
import mimetypes
import toml
from PIL import Image
from io import BytesIO

client = musicpd.MPDClient()
client.connect()

artfiles = []

for song in client.playlistinfo():
    path = song['file']
    slug = re.sub(r'[^a-z0-9]+', '-', song['title'].lower())[:16]
    if song['title'] == 'MEGALOVANIA':
        slug = 'ut-megalovania'
    elif song['title'] == 'MeGaLoVania':
        slug = 'hs-megalovania'
    print(slug)
    artfiles.append(slug)
    art = client.readpicture(path, 0)
    received = int(art['binary'])
    size = int(art['size'])
    ext = ".jpg"
    if art['type']:
        guessed = mimetypes.guess_extension(art['type'])
        if guessed:
            ext = guessed
    cover = bytearray()
    cover.extend(art.get('data'))
    while received < size:
        art = client.readpicture(path, received)
        cover.extend(art['data'])
        received += int(art['binary'])
    if received != size:
        print("mismatched size")
        break
    cover_image = Image.open(BytesIO(cover))
    (orig_width, orig_height) = cover_image.size
    (width, height) = (orig_width, orig_height)
    while width < 512 or height < 512:
        width += orig_width
        height += orig_height
    scaled = cover_image.resize((width, height), resample=Image.NEAREST)
    scaled.save(f'artfiles/{slug}{ext}')

client.disconnect()

with open('config.toml', 'w') as f:
    toml.dump({ 'artfiles': artfiles }, f)
