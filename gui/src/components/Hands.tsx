import React from 'react';
import styles from './styles/Hand.module.css';
import Hand from './Hand';


type HandsProps = {
    cardsArr: any[][],
    scores: number[]
  };

  const Hands: React.FC<HandsProps> = ({ cardsArr, scores }) => {
    return (
      <div className={styles.handsContainer}>
        
          {cardsArr.map((cards: any[], index: number) => {
            return (
                <div className={styles.handContainer}>
              <Hand title={`Player ${index} (${scores[index]})`} cards={cards}/>
                </div>

            );
          })}
      </div>
    );
  }

  export default Hands;