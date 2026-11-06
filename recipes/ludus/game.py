"""Star Cat: the drawing, the keys and the loop. The rules live in world.py.

Play:            python game.py
Sensor check:    python game.py --frames 180   (runs three seconds, then exits)
"""
import math
import sys

import pygame

from world import HEIGHT, WIDTH, World

TITLE = "Star Cat"
FPS = 60
SKY = (18, 22, 48)
CAT = (240, 160, 70)
STAR = (255, 220, 90)
TEXT = (235, 235, 245)


def star_points(cx, cy, r):
    points = []
    for i in range(10):
        radius = r if i % 2 == 0 else r * 0.45
        angle = -math.pi / 2 + i * math.pi / 5
        points.append((cx + radius * math.cos(angle), cy + radius * math.sin(angle)))
    return points


def draw_cat(screen, cat):
    x, y, s = cat.x, cat.y, cat.size
    pygame.draw.polygon(screen, CAT, [(x - s * 0.45, y - s * 0.2), (x - s * 0.3, y - s * 0.75), (x - s * 0.05, y - s * 0.35)])
    pygame.draw.polygon(screen, CAT, [(x + s * 0.45, y - s * 0.2), (x + s * 0.3, y - s * 0.75), (x + s * 0.05, y - s * 0.35)])
    pygame.draw.circle(screen, CAT, (int(x), int(y)), int(s / 2))
    for dx in (-0.18, 0.18):
        pygame.draw.circle(screen, SKY, (int(x + dx * s), int(y - s * 0.08)), max(2, int(s * 0.07)))


def draw(screen, font, world):
    screen.fill(SKY)
    for star in world.stars:
        pygame.draw.polygon(screen, STAR, star_points(star.x, star.y, star.size))
    draw_cat(screen, world.cat)
    hud = font.render(f"Stars: {world.score}    Lives: {world.lives - world.missed}", True, TEXT)
    screen.blit(hud, (16, 12))
    if world.over:
        msg = font.render("Game over. Press R to play again.", True, TEXT)
        screen.blit(msg, msg.get_rect(center=(WIDTH // 2, HEIGHT // 2)))


def main(argv):
    frames = None
    if "--frames" in argv:
        frames = int(argv[argv.index("--frames") + 1])

    pygame.init()
    screen = pygame.display.set_mode((WIDTH, HEIGHT))
    pygame.display.set_caption(TITLE)
    font = pygame.font.Font(None, 32)
    clock = pygame.time.Clock()
    world = World()

    count = 0
    running = True
    while running:
        dt = clock.tick(FPS) / 1000
        if frames is not None:
            dt = 1 / FPS  # fixed steps when checking, so the run is repeatable
        for event in pygame.event.get():
            if event.type == pygame.QUIT:
                running = False
            elif event.type == pygame.KEYDOWN and event.key == pygame.K_r and world.over:
                world = World()
        keys = pygame.key.get_pressed()
        world.step(
            dt,
            left=keys[pygame.K_LEFT] or keys[pygame.K_a],
            right=keys[pygame.K_RIGHT] or keys[pygame.K_d],
        )
        draw(screen, font, world)
        pygame.display.flip()
        count += 1
        if frames is not None and count >= frames:
            running = False

    pygame.quit()
    return 0


if __name__ == "__main__":
    sys.exit(main(sys.argv[1:]))
