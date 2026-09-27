"""What the installer fetches from the Internet: releases, places, airports."""
import urllib.request


def fetch(url):
    request = urllib.request.Request(url, headers={'User-Agent': 'cute-display-installer'})
    with urllib.request.urlopen(request) as response:
        return response.read()
