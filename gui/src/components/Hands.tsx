import React from 'react';
import styles from './styles/Hand.module.css';
import buttonStyles from './styles/Card.module.css'
import Hand from './Hand';


type HandsProps = {
    scores: number[],
    addresses: string[],
    kickButtonsState: any[],
    sitButtonsState: any[],
    cardsArr: any[][],
    kickEvent: any,
    sitEvent: any
  };

  const Hands: React.FC<HandsProps> = ({ scores, addresses, kickButtonsState,sitButtonsState, cardsArr, kickEvent, sitEvent }) => {
    const getKickButton = (index: number) => {
      if (!kickButtonsState[index].canBeKicked) {
        if (kickButtonsState[index].kickTimer === 0) {
          return (
            <button onClick={() => kickEvent(index)} disabled={true} className={buttonStyles.controlButton}>Kick</button>
          );
        }

        return (
          <button onClick={() => kickEvent(index)} disabled={true} className={buttonStyles.controlButton}>Kick (In {kickButtonsState[index].kickTimer}s)</button>
        );
      }
      if (kickButtonsState[index].kickTimer === 0) {
        return (
          <button onClick={() => kickEvent(index)} disabled={false} className={buttonStyles.controlButton}>Kick</button>
        );
      } else {
        return (
          <button onClick={() => kickEvent(index)} disabled={true} className={buttonStyles.controlButton}>Kick (In {kickButtonsState[index].kickTimer}s)</button>
        );
      }
    }

    return (
      <div className={styles.handsContainer}>
        
          {cardsArr.map((cards: any[], index: number) => {
            return (
                <div className={styles.handContainer}>
              <Hand title={addresses[index]} cards={cards} isDealer={false} dealerScore={0}/>
              <button disabled={true} className={buttonStyles.controlButton}>Score: {scores[index]}</button>
              {getKickButton(index)}
              <button onClick={() => sitEvent(index)} disabled={sitButtonsState[index].disabled} className={buttonStyles.controlButton}>Sit</button>
                </div>
                

            );
          })}
      </div>
    );
  }

  export default Hands;