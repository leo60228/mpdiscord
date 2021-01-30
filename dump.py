#!/usr/bin/env python3
import musicpd
import re
import mimetypes

client = musicpd.MPDClient()
client.connect()

for song in client.playlistinfo():
    path = song['file']
    slug = re.sub(r'[^a-z0-9]+', '-', song['title'].lower())
    print(slug)
    art = client.readpicture(path, 0)
    received = int(art['binary'])
    size = int(art['size'])
    ext = ".jpg"
    if art['type']:
        guessed = mimetypes.guess_extension(art['type'])
        if guessed:
            ext = guessed
    with open(f'artfiles/{slug}.jpg', 'wb') as cover:
        cover.write(art.get('data'))
        while received < size:
            art = client.readpicture(path, received)
            cover.write(art['data'])
            received += int(art['binary'])
        if received != size:
            print("mismatched size")
            break

client.disconnect()
