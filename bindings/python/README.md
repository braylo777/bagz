# bagz

Python bindings for the Bagz swipe-to-apply core. See the repository README for build steps.

```python
import bagz

deck = bagz.Deck.example()
deck.swipe("froth", "like")
deck.applied()
```
