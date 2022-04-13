# BlackJack over Secret Network

An implementation of a single-session BlackJack over Secret network.
In this game you will be able to double your money in less than a minute!

## Rules

1. The goal of blackjack is to beat the dealer's hand without going over 21.
2. Face cards are worth 10. Aces are worth 1 or 11, whichever makes a better hand.
3. Each player starts with two cards, one of the dealer's cards is hidden until the end.
4. To 'Hit' is to ask for another card. To 'Stand' is to hold your total and end your turn.
5. If you go over 21 you bust, and the dealer wins regardless of the dealer's hand.
6. If you are dealt 21 from the start (Ace & 10), you got a blackjack.
7. Blackjack means you win 1.5 the amount of your bet (WOW!!!!!)
8. Dealer will hit until his/her cards total 17 or higher.
9. Split can be done when you have two of the same card - the pair is split into two hands.
10. Splitting also doubles the bet, because each new hand is worth the original bet.
11. You can only split on the first move.
12. Aces can't be split.

# Design

## High Level Strategy

**_Note_**: In each and every time only one game session will be handled and only 6 seats will be available.
If the queue will not be empty the first person in the queue will be able to kick one of the players that played 20+ rounds in a row (Tx fees will be payed by the kicker). In addition, the players that are playing and the first player in the queue can kick a player who won't be responsive in his turn for more than 1 minute.

**_Note_**: A bet will be approved only by checking the Bank's balance, if the bank can't afford a bet the player who tries to bet will receive a message and will get the opportunity to make another bet.

## High Level Design

The game is built with 2 contracts in total.
The first contract (aka Bank), which is an administrative contract, will manage the funds of the game.
The money to the winner will be transferred by the bank and the money that was lost during the game will be transferred to the bank.
The second contract (aka Game) is the actual game manager it will interact both with the users (To manage the game session) and with the Bank to transfer money to and from the users.

The initial balance of the Bank will be 10 SCRT.

## Flows

### User entry

asdasda
