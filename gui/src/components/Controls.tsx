import React, { useState, useEffect } from 'react';
import styles from './styles/Controls.module.css';

type ControlsProps = {

  isFirstRound: boolean,
  isMyRound: boolean,
  isSeated: boolean,
  buttonState: any,
  betEvent: any,
  hitEvent: any,
  holdEvent: any,
  standEvent: any,
  lastScoreEvent: any,
};

const Controls: React.FC<ControlsProps> = ({ isFirstRound, isMyRound, isSeated, buttonState, betEvent, hitEvent, holdEvent, standEvent, lastScoreEvent }) => {
  const [amount, setAmount] = useState(1);
  const [inputStyle, setInputStyle] = useState(styles.input);

  useEffect(() => {
    validation();
  }, [amount]);

  const validation = () => {
    if (amount < 1) {
      setInputStyle(styles.inputError);
      return false;
    }

    if (Math.floor(amount).toString() !== amount.toString()) {
      setInputStyle(styles.inputError);
      return false;
    }

    setInputStyle(styles.input);
    return true;
  }

  const amountChange = (e: any) => {
    setAmount(e.target.value);
  }

  const onBetClick = () => {
    if (validation()) {
      betEvent(Math.round(amount * 100) / 100);
    }
  }

  const getControls = () => {
    if(!isMyRound) {
      if (isSeated) {
        return (
          <div className={styles.controlsContainer}>
            <button onClick={() => standEvent()} disabled={buttonState.standDisabled} className={styles.button}>Stand</button>
            <button onClick={() => lastScoreEvent()} disabled={buttonState.standDisabled} className={styles.button}>Last Score</button>
          </div>
        );
      }

      return (
        <div className={styles.controlsContainer}>
        </div>
      );
      
    } 
    if (isFirstRound) {
      return (
        <div className={styles.controlsContainer}>
          <div className={styles.betContainer}>
            <h4>Amount:</h4>
            <input autoFocus type='number' value={amount} onChange={amountChange} className={inputStyle} />
          </div>
          <button onClick={() => onBetClick()} className={styles.button}>Bet</button>
          <button onClick={() => standEvent()} disabled={buttonState.standDisabled} className={styles.button}>Stand</button>
          <button onClick={() => lastScoreEvent()} disabled={buttonState.standDisabled} className={styles.button}>Last Score</button>
          
        </div>
      );
    }
    else {
      return (
        <div className={styles.controlsContainer}>
          <button onClick={() => hitEvent()} disabled={buttonState.hitDisabled} className={styles.button}>Hit</button>
          <button onClick={() => holdEvent()} disabled={buttonState.holdDisabled} className={styles.button}>Hold</button>
          <button onClick={() => standEvent()} disabled={buttonState.standDisabled} className={styles.button}>Stand</button>
          <button onClick={() => lastScoreEvent()} disabled={buttonState.standDisabled} className={styles.button}>Last Score</button>
        </div>
      );
    }
  }

  return (
    <>
      {getControls()}
    </>
  );
}

export default Controls;