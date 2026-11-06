"""The rules of the game. No drawing here, so every rule can be tested.

A cat moves left and right along the bottom. Stars fall. Catch them for points.
Miss too many and the game is over.
"""
import random
from dataclasses import dataclass, field

WIDTH, HEIGHT = 800, 600


@dataclass
class Cat:
    x: float = WIDTH / 2
    y: float = HEIGHT - 60
    speed: float = 360.0  # pixels per second
    size: float = 44.0


@dataclass
class Star:
    x: float
    y: float
    speed: float
    size: float = 14.0


@dataclass
class World:
    cat: Cat = field(default_factory=Cat)
    stars: list = field(default_factory=list)
    score: int = 0
    missed: int = 0
    lives: int = 3
    spawn_every: float = 0.9  # seconds between stars
    since_spawn: float = 0.0
    rng: random.Random = field(default_factory=lambda: random.Random(7))

    @property
    def over(self) -> bool:
        return self.missed >= self.lives

    def spawn(self) -> Star:
        # Spawn: a new star appears at a random x, just above the screen.
        star = Star(x=self.rng.uniform(30, WIDTH - 30), y=-20, speed=self.rng.uniform(140, 240) + self.score * 4)
        self.stars.append(star)
        return star

    def caught(self, star: Star) -> bool:
        # Collision detection: does this star touch the cat?
        reach = self.cat.size / 2 + star.size
        return abs(star.x - self.cat.x) < reach and abs(star.y - self.cat.y) < reach

    def step(self, dt: float, left: bool = False, right: bool = False) -> None:
        """The update: advance the world by dt (delta time) seconds."""
        if self.over:
            return
        direction = int(right) - int(left)
        half = self.cat.size / 2
        self.cat.x = min(WIDTH - half, max(half, self.cat.x + direction * self.cat.speed * dt))

        self.since_spawn += dt
        if self.since_spawn >= self.spawn_every:
            self.since_spawn = 0.0
            self.spawn()

        kept = []
        for star in self.stars:
            star.y += star.speed * dt
            if self.caught(star):
                self.score += 1
            elif star.y > HEIGHT + star.size:
                self.missed += 1
            else:
                kept.append(star)
        self.stars = kept
