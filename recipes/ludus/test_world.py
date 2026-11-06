"""Tests for the rules. Run with: python -m pytest -q"""
from world import HEIGHT, WIDTH, Star, World


def test_cat_moves_and_stays_on_screen():
    w = World()
    start = w.cat.x
    w.step(0.1, right=True)
    assert w.cat.x > start
    for _ in range(200):
        w.step(0.1, right=True)
    assert w.cat.x <= WIDTH - w.cat.size / 2
    for _ in range(400):
        w.step(0.1, left=True)
    assert w.cat.x >= w.cat.size / 2


def test_catching_a_star_scores():
    w = World()
    w.stars = [Star(x=w.cat.x, y=w.cat.y - 5, speed=0)]
    w.step(0.01)
    assert w.score == 1
    assert w.stars == []


def test_missing_stars_ends_the_game():
    w = World()
    w.cat.x = 40
    for _ in range(w.lives):
        w.stars.append(Star(x=WIDTH - 40, y=HEIGHT + 100, speed=0))
    w.step(0.01)
    assert w.missed == w.lives
    assert w.over
    score = w.score
    w.step(1.0, right=True)  # nothing moves once the game is over
    assert w.score == score


def test_stars_keep_coming():
    w = World()
    for _ in range(60):
        w.step(0.1)
    assert w.score + w.missed + len(w.stars) >= 5
