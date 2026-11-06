# Quests

Each quest is a change to Star Cat that teaches a few proper words. Ask Valla for it in your own words, but use the new words. Then read the table: what you asked, what Valla takes it to mean, what nobody said. Approve it, play it, move on.

See every word, and which ones you have used, with `valla words`.

---

**1. Change a number**
Words: *variable*, *value*
Vague: "make the cat faster"
Precise: "change the cat's speed variable from 360 to 500"

**2. Change a color**
Words: *constant*, *color*
Precise: "change the STAR color constant to light blue, (120, 200, 255)"

**3. More lives**
Words: *attribute*, *state*
Precise: "set the lives attribute of World to 5"

**4. A title on the HUD**
Words: *HUD*, *render*
Precise: "render the text Star Cat in the top right of the HUD"

**5. Speed up as you score**
Words: *condition*, *delta time*
Precise: "in World.step, every time the score reaches a multiple of 10, make spawn_every 10 percent smaller, with a condition so it never goes below 0.3"

**6. A bad star**
Words: *class*, *object*, *collision*
Precise: "add a BadStar class, drawn red; spawn one object of it for every five stars; a collision with a BadStar costs a life"

**7. Remember the high score**
Words: *function*, *return*, *import*
Precise: "add a function that loads the high score from a file and returns it, and one that saves it when the game is over; import json to do it"

**8. A title screen**
Words: *state*, *event*, *game loop*
Precise: "add a title state before the game starts; the game loop shows the title until the space key event"

**9. Jumping**
Words: *coordinates*, *update*, *input*
Precise: "on the up arrow input, the cat jumps: give it a y velocity attribute, and in update apply gravity until it lands back at its starting y coordinate"

**10. A second player**
Words: *list*, *loop*, *parameter*
Precise: "add a second cat on the A and D keys; keep both cats in a list, and loop over the list to move and draw them; draw_cat already takes the cat as a parameter"

**11. Make your own test**
Words: *test*, *bug*
Precise: "add a test to test_world.py that checks a BadStar collision costs exactly one life"

**12. Your game now**
Words: all of them
Change Star Cat into the game you actually want. A fish catching bubbles. A ship dodging rocks. You have the words now.

---

If something breaks, nothing is lost: `valla restore` returns the game to the last version that worked.
