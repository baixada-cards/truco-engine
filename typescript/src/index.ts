export const MATCH_TARGET = 12
export const INITIAL_HAND_VALUE = 1
export const RAISE_LADDER = [1, 3, 6, 9, 12] as const
const ELEVEN_AUTO_HAND_VALUE = 3

export type Player = 0 | 1
export type Rank = '4' | '5' | '6' | '7' | 'Q' | 'J' | 'K' | 'A' | '2' | '3'
export type Suit = 'DIAMONDS' | 'SPADES' | 'HEARTS' | 'CLUBS'
export type Visibility = 'up' | 'down'

export interface Score {
  0: number
  1: number
}

export interface Card {
  id: string
  rank: Rank
  suit: Suit
}

export interface Turnup {
  rank: Rank
  suit: Suit
}

export interface PlayedCard {
  player: Player
  visibility: Visibility
  card: Card
}

export interface CompletedRound {
  leader: Player
  winner: Player | null
  plays?: PlayedCard[]
}

export interface CurrentRound {
  leader: Player
  plays: PlayedCard[]
}

export interface PendingRaise {
  raised_by: Player
  to: number
  previous_value: number
}

export interface PendingDecision {
  type: 'mao_de_onze'
  player: Player
}

export interface Hands {
  0: Card[]
  1: Card[]
}

export interface GameState {
  dealer: Player
  next_player: Player | null
  score: Score
  hand_value: number
  turnup: Turnup
  hands: Hands
  completed_rounds: CompletedRound[]
  current_round: CurrentRound
  last_raised_by?: Player | null
  pending_raise: PendingRaise | null
  pending_decision?: PendingDecision | null
}

export type Action =
  | { type: 'play_face_up'; card_id: string }
  | { type: 'play_face_down'; card_id: string }
  | { type: 'raise'; to: number }
  | { type: 'accept_raise' }
  | { type: 'fold' }
  | { type: 'accept_eleven' }
  | { type: 'fold_eleven' }
  | { type: 'concede_hand' }

export interface PublicCard {
  rank: Rank
  suit: Suit
}

export interface PublicPlay {
  player: Player
  visibility: Visibility
  card?: PublicCard
}

export interface PublicRound {
  leader: Player
  plays: PublicPlay[]
}

export interface PublicCompletedRound {
  leader: Player
  winner: Player | null
}

export interface PublicDecision {
  type: 'mao_de_onze'
  player: Player
}

export interface PublicState {
  next_player: Player | null
  hand_value: number
  hand_winner: Player | null
  match_winner: Player | null
  score: Score
  completed_rounds: PublicCompletedRound[]
  current_round: PublicRound
  pending_raise: PendingRaise | null
  pending_decision: PublicDecision | null
}

export interface EngineState {
  state: GameState
  hand_winner: Player | null
  match_winner: Player | null
}

export interface PlayerHandView {
  public_state: PublicState
  hand: Card[]
}

export type EngineErrorCode =
  | 'INVALID_INITIAL_STATE'
  | 'HAND_ALREADY_DECIDED'
  | 'NO_ACTIVE_HAND'
  | 'HAND_STILL_IN_PROGRESS'
  | 'MATCH_ALREADY_DECIDED'
  | 'ACT_OUT_OF_TURN'
  | 'CARD_NOT_IN_HAND'
  | 'HIDE_NOT_ALLOWED_IN_FIRST_ROUND'
  | 'RAISE_NOT_ALLOWED'
  | 'INVALID_RAISE_TARGET'
  | 'NO_PENDING_RAISE'
  | 'NO_PENDING_ELEVEN_DECISION'
  | 'RAISE_PENDING'
  | 'DECISION_PENDING'
  | 'INVALID_PLAYER'
  | 'SERIALIZATION_FAILED'

const ERROR_MESSAGES: Record<EngineErrorCode, string> = {
  INVALID_INITIAL_STATE: 'invalid initial state',
  HAND_ALREADY_DECIDED: 'hand already decided',
  NO_ACTIVE_HAND: 'no active hand',
  HAND_STILL_IN_PROGRESS: 'hand still in progress',
  MATCH_ALREADY_DECIDED: 'match already decided',
  ACT_OUT_OF_TURN: 'act out of turn',
  CARD_NOT_IN_HAND: 'card not in hand',
  HIDE_NOT_ALLOWED_IN_FIRST_ROUND: 'hide not allowed in first round',
  RAISE_NOT_ALLOWED: 'raise not allowed',
  INVALID_RAISE_TARGET: 'invalid raise target',
  NO_PENDING_RAISE: 'no pending raise',
  NO_PENDING_ELEVEN_DECISION: 'no pending eleven decision',
  RAISE_PENDING: 'a pending raise must be answered first',
  DECISION_PENDING: 'a pending decision must be answered first',
  INVALID_PLAYER: 'invalid player',
  SERIALIZATION_FAILED: 'serialization failed',
}

export class TrucoEngineError extends Error {
  readonly code: EngineErrorCode

  constructor(code: EngineErrorCode) {
    super(ERROR_MESSAGES[code])
    this.name = 'TrucoEngineError'
    this.code = code
  }
}

export interface EngineFixture {
  fixture_version: 'engine-fixture/v1'
  id: string
  ruleset: 'truco-2p-v1'
  description: string
  initial_state: GameState
  steps?: FixtureStep[]
  expect_initial_state_error?: EngineErrorCode
}

export type FixtureStep =
  | {
    op: 'assert_legal_actions'
    player: Player
    must_include?: Action[]
    must_exclude?: Action[]
  }
  | { op: 'apply_action'; player: Player; action: Action }
  | { op: 'assert_state'; expect: unknown }
  | { op: 'assert_export_round_trip'; expect: unknown }
  | {
    op: 'assert_rejected_action'
    player: Player
    action: Action
    error_code: EngineErrorCode
  }

export interface FixtureRunReport {
  fixture_id: string
  status: 'pass' | 'fail'
  message: string
}

export class Engine {
  private readonly firstRoundLeader: Player
  private stateValue: GameState
  private handWinnerValue: Player | null
  private matchWinnerValue: Player | null

  private constructor(
    state: GameState,
    firstRoundLeaderValue: Player,
    handWinner: Player | null,
    matchWinner: Player | null,
  ) {
    this.stateValue = state
    this.firstRoundLeader = firstRoundLeaderValue
    this.handWinnerValue = handWinner
    this.matchWinnerValue = matchWinner
  }

  static newHand(dealer: Player, score: Score, turnup: Turnup, hands: Hands): Engine {
    const leader = otherPlayer(dealer)
    const handValue = startingHandValue(score)
    const decisionPlayer = elevenHandDecisionPlayer(score)
    const pendingDecision = decisionPlayer == null
      ? null
      : { type: 'mao_de_onze' as const, player: decisionPlayer }
    const nextPlayer = pendingDecision?.player ?? leader

    return Engine.fromState({
      dealer,
      next_player: nextPlayer,
      score: clone(score),
      hand_value: handValue,
      turnup: clone(turnup),
      hands: clone(hands),
      completed_rounds: [],
      current_round: { leader, plays: [] },
      last_raised_by: null,
      pending_raise: null,
      pending_decision: pendingDecision,
    })
  }

  static fromState(state: GameState): Engine {
    const normalized = normalizeState(state)
    validateNormalizedState(normalized)
    return new Engine(normalized, firstRoundLeader(normalized.dealer, normalized.completed_rounds), null, null)
  }

  static fromExportedState(exported: EngineState): Engine {
    const normalized = normalizeState(exported.state)
    validateExportedNormalizedState(normalized, exported.hand_winner, exported.match_winner)
    const matchWinner = scoreWinner(normalized.score)
    if (!sameOptionalPlayer(exported.match_winner, matchWinner)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }

    return new Engine(
      normalized,
      firstRoundLeader(normalized.dealer, normalized.completed_rounds),
      exported.hand_winner ?? null,
      matchWinner,
    )
  }

  currentPlayer(): Player | null {
    return this.stateValue.next_player
  }

  handWinner(): Player | null {
    return this.handWinnerValue
  }

  matchWinner(): Player | null {
    return this.matchWinnerValue
  }

  isHandOver(): boolean {
    return this.handWinnerValue != null
  }

  isMatchOver(): boolean {
    return this.matchWinnerValue != null
  }

  legalActionsForCurrentPlayer(): Action[] {
    const player = this.currentPlayer()
    if (player == null) {
      throw new TrucoEngineError('HAND_ALREADY_DECIDED')
    }
    return this.legalActions(player)
  }

  strategicLegalActionsForCurrentPlayer(): Action[] {
    return filterStrategicActions(this.legalActionsForCurrentPlayer())
  }

  legalActions(player: Player): Action[] {
    this.ensureActiveTurn(player)

    const decision = this.stateValue.pending_decision ?? null
    if (decision != null) {
      if (decision.player !== player) {
        throw new TrucoEngineError('ACT_OUT_OF_TURN')
      }
      return [
        { type: 'accept_eleven' },
        { type: 'fold_eleven' },
        { type: 'concede_hand' },
      ]
    }

    const pendingRaise = this.stateValue.pending_raise
    if (pendingRaise != null) {
      const actions: Action[] = [
        { type: 'accept_raise' },
        { type: 'fold' },
        { type: 'concede_hand' },
      ]
      if (this.reRaiseAvailable(player)) {
        const nextValue = nextRaiseAfter(pendingRaise.to)
        if (nextValue == null) {
          throw new TrucoEngineError('RAISE_NOT_ALLOWED')
        }
        actions.push({ type: 'raise', to: nextValue })
      }
      return actions
    }

    const actions: Action[] = []
    for (const card of playerHand(this.stateValue, player)) {
      actions.push({ type: 'play_face_up', card_id: card.id })
      if (this.hiddenPlayAllowed()) {
        actions.push({ type: 'play_face_down', card_id: card.id })
      }
    }

    if (this.raiseAvailable(player)) {
      const nextValue = nextRaiseAfter(this.stateValue.hand_value)
      if (nextValue != null) {
        actions.push({ type: 'raise', to: nextValue })
      }
    }
    actions.push({ type: 'concede_hand' })

    return actions
  }

  strategicLegalActions(player: Player): Action[] {
    return filterStrategicActions(this.legalActions(player))
  }

  applyActionForCurrentPlayer(action: Action): Player {
    const player = this.currentPlayer()
    if (player == null) {
      throw new TrucoEngineError('HAND_ALREADY_DECIDED')
    }
    this.applyAction(player, action)
    return player
  }

  applyAction(player: Player, action: Action): void {
    this.ensureActiveTurn(player)

    switch (action.type) {
      case 'play_face_up':
        this.applyPlay(player, action.card_id, 'up')
        return
      case 'play_face_down':
        this.applyPlay(player, action.card_id, 'down')
        return
      case 'raise':
        this.applyRaise(player, action.to)
        return
      case 'accept_raise':
        this.applyAcceptRaise()
        return
      case 'fold':
        this.applyFold()
        return
      case 'accept_eleven':
        this.applyAcceptEleven()
        return
      case 'fold_eleven':
        this.applyFoldEleven()
        return
      case 'concede_hand':
        this.applyConcedeHand(player)
        return
      default:
        throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }

  publicState(): PublicState {
    return {
      next_player: this.stateValue.next_player,
      hand_value: this.stateValue.hand_value,
      hand_winner: this.handWinnerValue,
      match_winner: this.matchWinnerValue,
      score: clone(this.stateValue.score),
      completed_rounds: this.stateValue.completed_rounds.map((round) => ({
        leader: round.leader,
        winner: round.winner ?? null,
      })),
      current_round: {
        leader: this.stateValue.current_round.leader,
        plays: this.stateValue.current_round.plays.map((play) => {
          const publicPlay: PublicPlay = {
            player: play.player,
            visibility: play.visibility,
          }
          if (play.visibility === 'up') {
            publicPlay.card = {
              rank: play.card.rank,
              suit: play.card.suit,
            }
          }
          return publicPlay
        }),
      },
      pending_raise: this.stateValue.pending_raise == null ? null : clone(this.stateValue.pending_raise),
      pending_decision: this.stateValue.pending_decision == null
        ? null
        : {
          type: this.stateValue.pending_decision.type,
          player: this.stateValue.pending_decision.player,
        },
    }
  }

  publicStateValue(): unknown {
    return this.publicState()
  }

  playerView(player: Player): PlayerHandView {
    if (!isPlayer(player)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }

    return {
      public_state: this.publicState(),
      hand: clone(playerHand(this.stateValue, player)),
    }
  }

  exportState(): EngineState {
    return {
      state: clone(this.stateValue),
      hand_winner: this.handWinnerValue,
      match_winner: this.matchWinnerValue,
    }
  }

  state(): GameState {
    return clone(this.stateValue)
  }

  private ensureActiveTurn(player: Player): void {
    if (this.handWinnerValue != null) {
      throw new TrucoEngineError('HAND_ALREADY_DECIDED')
    }

    if (this.stateValue.next_player === player) {
      return
    }

    throw new TrucoEngineError('ACT_OUT_OF_TURN')
  }

  private hiddenPlayAllowed(): boolean {
    return this.stateValue.completed_rounds.length > 0
  }

  private raiseAvailable(player: Player): boolean {
    return this.stateValue.pending_raise == null
      && this.stateValue.pending_decision == null
      && this.stateValue.hand_value < MATCH_TARGET
      && !scoreTriggersElevenHand(this.stateValue.score)
      && this.stateValue.last_raised_by !== player
  }

  private reRaiseAvailable(player: Player): boolean {
    const pendingRaise = this.stateValue.pending_raise
    return pendingRaise != null
      && nextRaiseAfter(pendingRaise.to) != null
      && pendingRaise.raised_by !== player
      && this.stateValue.last_raised_by !== player
  }

  private applyPlay(player: Player, cardId: string, visibility: Visibility): void {
    if (this.stateValue.pending_decision != null) {
      throw new TrucoEngineError('DECISION_PENDING')
    }
    if (this.stateValue.pending_raise != null) {
      throw new TrucoEngineError('RAISE_PENDING')
    }
    if (visibility === 'down' && !this.hiddenPlayAllowed()) {
      throw new TrucoEngineError('HIDE_NOT_ALLOWED_IN_FIRST_ROUND')
    }

    const hand = mutablePlayerHand(this.stateValue, player)
    const cardIndex = hand.findIndex((card) => card.id === cardId)
    if (cardIndex < 0) {
      throw new TrucoEngineError('CARD_NOT_IN_HAND')
    }
    const [card] = hand.splice(cardIndex, 1)
    this.stateValue.current_round.plays.push({ player, visibility, card: card! })

    if (this.stateValue.current_round.plays.length === 1) {
      this.stateValue.next_player = otherPlayer(player)
      return
    }

    this.resolveRound()
  }

  private applyRaise(player: Player, to: number): void {
    if (this.stateValue.pending_decision != null) {
      throw new TrucoEngineError('RAISE_NOT_ALLOWED')
    }
    if (scoreTriggersElevenHand(this.stateValue.score)) {
      throw new TrucoEngineError('RAISE_NOT_ALLOWED')
    }

    const pendingRaise = this.stateValue.pending_raise
    if (pendingRaise != null) {
      if (!this.reRaiseAvailable(player)) {
        throw new TrucoEngineError('RAISE_NOT_ALLOWED')
      }
      const expected = nextRaiseAfter(pendingRaise.to)
      if (expected == null) {
        throw new TrucoEngineError('RAISE_NOT_ALLOWED')
      }
      if (to !== expected) {
        throw new TrucoEngineError('INVALID_RAISE_TARGET')
      }

      this.stateValue.pending_raise = {
        raised_by: player,
        to,
        previous_value: pendingRaise.to,
      }
      this.stateValue.last_raised_by = player
      this.stateValue.next_player = otherPlayer(player)
      return
    }

    if (!this.raiseAvailable(player)) {
      throw new TrucoEngineError('RAISE_NOT_ALLOWED')
    }

    const expected = nextRaiseAfter(this.stateValue.hand_value)
    if (expected == null) {
      throw new TrucoEngineError('RAISE_NOT_ALLOWED')
    }
    if (to !== expected) {
      throw new TrucoEngineError('INVALID_RAISE_TARGET')
    }

    this.stateValue.pending_raise = {
      raised_by: player,
      to,
      previous_value: this.stateValue.hand_value,
    }
    this.stateValue.last_raised_by = player
    this.stateValue.next_player = otherPlayer(player)
  }

  private applyAcceptRaise(): void {
    const pendingRaise = this.stateValue.pending_raise
    if (pendingRaise == null) {
      throw new TrucoEngineError('NO_PENDING_RAISE')
    }
    this.stateValue.pending_raise = null
    this.stateValue.hand_value = pendingRaise.to
    this.stateValue.next_player = this.stateValue.current_round.plays.length === 0
      ? this.stateValue.current_round.leader
      : otherPlayer(this.stateValue.current_round.plays[0]!.player)
  }

  private applyFold(): void {
    const pendingRaise = this.stateValue.pending_raise
    if (pendingRaise == null) {
      throw new TrucoEngineError('NO_PENDING_RAISE')
    }
    this.stateValue.pending_raise = null
    this.endHand(pendingRaise.raised_by, pendingRaise.previous_value)
  }

  private applyAcceptEleven(): void {
    if (this.stateValue.pending_decision == null) {
      throw new TrucoEngineError('NO_PENDING_ELEVEN_DECISION')
    }
    this.stateValue.pending_decision = null
    this.stateValue.hand_value = ELEVEN_AUTO_HAND_VALUE
    this.stateValue.next_player = this.stateValue.current_round.leader
  }

  private applyFoldEleven(): void {
    const decision = this.stateValue.pending_decision
    if (decision == null) {
      throw new TrucoEngineError('NO_PENDING_ELEVEN_DECISION')
    }
    this.stateValue.pending_decision = null
    this.endHand(otherPlayer(decision.player), elevenFoldAward())
  }

  private applyConcedeHand(player: Player): void {
    // A pending re-raise implicitly accepted `previous_value`, so a
    // concession pays the same as a fold there — never less.
    const awarded = this.stateValue.pending_raise?.previous_value ?? this.stateValue.hand_value
    this.endHand(otherPlayer(player), awarded)
  }

  private resolveRound(): void {
    const plays = this.stateValue.current_round.plays
    const winner = comparePlays(this.stateValue.turnup, plays[0]!, plays[1]!)
    const leader = this.stateValue.current_round.leader
    this.stateValue.completed_rounds.push({
      leader,
      winner,
      plays: clone(plays),
    })
    this.stateValue.current_round.plays = []
    this.stateValue.current_round.leader = winner ?? leader

    const handWinner = decideHandWinner(this.stateValue.completed_rounds, this.firstRoundLeader)
    if (handWinner != null) {
      this.endHand(handWinner, this.stateValue.hand_value)
      return
    }

    this.stateValue.next_player = this.stateValue.current_round.leader
  }

  private endHand(winner: Player, awardedValue: number): void {
    this.handWinnerValue = winner
    this.stateValue.pending_raise = null
    this.stateValue.pending_decision = null
    this.stateValue.next_player = null

    if (winner === 0) {
      this.stateValue.score[0] = Math.min(MATCH_TARGET, this.stateValue.score[0] + awardedValue)
    } else {
      this.stateValue.score[1] = Math.min(MATCH_TARGET, this.stateValue.score[1] + awardedValue)
    }

    if (this.stateValue.score[0] >= MATCH_TARGET) {
      this.matchWinnerValue = 0
    } else if (this.stateValue.score[1] >= MATCH_TARGET) {
      this.matchWinnerValue = 1
    }
  }
}

export function validateState(state: GameState): void {
  validateNormalizedState(normalizeState(state))
}

export function executeFixture(fixture: EngineFixture): FixtureRunReport {
  try {
    const message = executeFixtureInner(fixture)
    return {
      fixture_id: fixture.id,
      status: 'pass',
      message,
    }
  } catch (error) {
    return {
      fixture_id: fixture.id,
      status: 'fail',
      message: error instanceof Error ? error.message : String(error),
    }
  }
}

export function assertExpectedSubset(actual: unknown, expected: unknown): void {
  const message = expectedSubsetMismatch(actual, expected)
  if (message != null) {
    throw new Error(message)
  }
}

function executeFixtureInner(fixture: EngineFixture): string {
  if (fixture.expect_initial_state_error != null) {
    try {
      validateState(fixture.initial_state)
    } catch (error) {
      const code = engineErrorCode(error)
      if (code === fixture.expect_initial_state_error) {
        return `initial state rejected with ${fixture.expect_initial_state_error}`
      }
      throw new Error(`expected initial state error ${fixture.expect_initial_state_error}, got ${code ?? String(error)}`)
    }
    throw new Error(`expected initial state error ${fixture.expect_initial_state_error}, but state was accepted`)
  }

  const engine = Engine.fromState(fixture.initial_state)
  const steps = fixture.steps ?? []
  for (const [index, step] of steps.entries()) {
    try {
      executeFixtureStep(engine, step)
    } catch (error) {
      const message = error instanceof Error ? error.message : String(error)
      throw new Error(`step ${index + 1} failed: ${message}`)
    }
  }

  return `executed ${steps.length} step(s)`
}

function executeFixtureStep(engine: Engine, step: FixtureStep): void {
  switch (step.op) {
    case 'assert_legal_actions': {
      let actions: Action[]
      try {
        actions = engine.legalActions(step.player)
      } catch (error) {
        const code = engineErrorCode(error) ?? String(error)
        throw new Error(`legal_actions returned ${code}`)
      }

      for (const action of step.must_include ?? []) {
        if (!actions.some((candidate) => actionEquals(candidate, action))) {
          throw new Error(`missing expected legal action ${JSON.stringify(action)}`)
        }
      }
      for (const action of step.must_exclude ?? []) {
        if (actions.some((candidate) => actionEquals(candidate, action))) {
          throw new Error(`unexpectedly allowed action ${JSON.stringify(action)}`)
        }
      }
      return
    }
    case 'apply_action':
      try {
        engine.applyAction(step.player, step.action)
      } catch (error) {
        const code = engineErrorCode(error) ?? String(error)
        throw new Error(`apply_action returned ${code}`)
      }
      return
    case 'assert_state': {
      const actual = engine.publicStateValue()
      const message = expectedSubsetMismatch(actual, step.expect)
      if (message != null) {
        throw new Error(
          `state mismatch: ${message}\nexpected subset: ${prettyJson(step.expect)}\nactual: ${prettyJson(actual)}`,
        )
      }
      return
    }
    case 'assert_export_round_trip': {
      const before = engine.publicStateValue()
      const restored = Engine.fromExportedState(engine.exportState())
      const after = restored.publicStateValue()

      if (!jsonEquals(before, after)) {
        throw new Error(`public state changed after round-trip\nbefore: ${prettyJson(before)}\nafter: ${prettyJson(after)}`)
      }

      const message = expectedSubsetMismatch(after, step.expect)
      if (message != null) {
        throw new Error(
          `round-trip state mismatch: ${message}\nexpected subset: ${prettyJson(step.expect)}\nactual: ${prettyJson(after)}`,
        )
      }
      return
    }
    case 'assert_rejected_action':
      try {
        engine.applyAction(step.player, step.action)
      } catch (error) {
        const code = engineErrorCode(error)
        if (code === step.error_code) {
          return
        }
        throw new Error(`expected rejected action ${step.error_code}, got ${code ?? String(error)}`)
      }
      throw new Error(`expected rejected action ${JSON.stringify(step.action)}, but it succeeded`)
    default:
      throw new Error(`unsupported fixture step ${(step as { op: string }).op}`)
  }
}

function normalizeState(input: GameState): GameState {
  const state = clone(input)
  if (state.last_raised_by == null) {
    state.last_raised_by = state.pending_raise?.raised_by ?? null
  }
  if (state.hand_value === 1 && state.pending_raise == null) {
    state.last_raised_by = null
  }

  for (const play of state.current_round.plays) {
    const hand = mutablePlayerHand(state, play.player)
    const index = hand.findIndex((card) => card.id === play.card.id)
    if (index >= 0) {
      hand.splice(index, 1)
    }
  }

  return state
}

function validateExportedNormalizedState(
  state: GameState,
  handWinner: Player | null,
  matchWinner: Player | null,
): void {
  if (handWinner != null) {
    validateFinishedNormalizedState(state, handWinner, matchWinner)
    return
  }

  validateNormalizedState(state)
  if (scoreWinner(state.score) != null || matchWinner != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
}

function validateNormalizedState(state: GameState): void {
  validateSharedNormalizedState(state)

  // A live hand cannot exist once the match is decided.
  if (scoreWinner(state.score) != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  // Raises are forbidden for the whole hand whenever any player is on 11,
  // and a one-player-at-11 hand is worth 1 only while the decision is open
  // (accepting makes it 3; folding ends it).
  if (scoreTriggersElevenHand(state.score) && state.pending_raise != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (elevenHandDecisionPlayer(state.score) != null) {
    const expectedValue = state.pending_decision != null ? INITIAL_HAND_VALUE : ELEVEN_AUTO_HAND_VALUE
    if (state.hand_value !== expectedValue) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }

  if (bothPlayersOnEleven(state.score)) {
    if (state.hand_value !== startingHandValue(state.score)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
    if (state.pending_decision != null) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }

  const expectedLeader = expectedRoundLeader(state.dealer, state.completed_rounds)
  if (state.current_round.leader !== expectedLeader) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const firstPlay = state.current_round.plays[0]
  if (firstPlay != null && firstPlay.player !== state.current_round.leader) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const pendingRaise = state.pending_raise
  if (pendingRaise != null) {
    if (!raiseValueReachableFrom(state.hand_value, pendingRaise.previous_value)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
    if (!isPlayer(pendingRaise.raised_by)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
    if (state.last_raised_by !== pendingRaise.raised_by) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
    const expectedTo = nextRaiseAfter(pendingRaise.previous_value)
    if (expectedTo == null || pendingRaise.to !== expectedTo) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  } else if (state.hand_value === 1) {
    if (state.last_raised_by != null) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  } else if (state.last_raised_by == null && !scoreTriggersElevenHand(state.score)) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const decision = state.pending_decision
  if (decision != null) {
    const expectedDecisionPlayer = elevenHandDecisionPlayer(state.score)
    if (
      expectedDecisionPlayer == null
      || decision.player !== expectedDecisionPlayer
      || state.completed_rounds.length > 0
      || state.current_round.plays.length > 0
    ) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }

  if (decideHandWinner(state.completed_rounds, firstRoundLeader(state.dealer, state.completed_rounds)) != null) {
    throw new TrucoEngineError('HAND_ALREADY_DECIDED')
  }

  let expectedNextPlayer: Player | null
  if (decision != null) {
    expectedNextPlayer = decision.player
  } else if (pendingRaise != null) {
    expectedNextPlayer = otherPlayer(pendingRaise.raised_by)
  } else if (state.current_round.plays.length === 0) {
    expectedNextPlayer = state.current_round.leader
  } else {
    expectedNextPlayer = otherPlayer(state.current_round.plays[0]!.player)
  }

  if (state.next_player !== expectedNextPlayer) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
}

function validateFinishedNormalizedState(
  state: GameState,
  handWinner: Player,
  matchWinner: Player | null,
): void {
  validateSharedNormalizedState(state)

  if (!isPlayer(handWinner) || state.next_player != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (state.pending_decision != null || state.pending_raise != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const expectedLeader = expectedRoundLeader(state.dealer, state.completed_rounds)
  if (state.current_round.leader !== expectedLeader) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const firstPlay = state.current_round.plays[0]
  if (firstPlay != null && firstPlay.player !== state.current_round.leader) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  const resolvedWinner = decideHandWinner(
    state.completed_rounds,
    firstRoundLeader(state.dealer, state.completed_rounds),
  )
  if (resolvedWinner != null && resolvedWinner !== handWinner) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  if (!sameOptionalPlayer(matchWinner, scoreWinner(state.score))) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
}

function validateSharedNormalizedState(state: GameState): void {
  if (
    !isPlayer(state.dealer)
    || !isPlayer(state.current_round.leader)
    || (state.next_player != null && !isPlayer(state.next_player))
    || (state.last_raised_by != null && !isPlayer(state.last_raised_by))
  ) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }

  if (state.score[0] > MATCH_TARGET || state.score[1] > MATCH_TARGET) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (!(RAISE_LADDER as readonly number[]).includes(state.hand_value)) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (state.completed_rounds.length > 3 || state.current_round.plays.length > 1) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (state.pending_decision != null && state.pending_raise != null) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (state.current_round.plays.some((play) => !isPlayer(play.player))) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (state.completed_rounds.some((round) => (
    !isPlayer(round.leader)
    || (round.winner != null && !isPlayer(round.winner))
    || !validCompletedRoundHistory(round, state)
  ))) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  if (
    state.completed_rounds.length === 0
    && state.current_round.plays.some((play) => play.visibility === 'down')
  ) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  validateRoundLeaderChain(state)
  if (hasDuplicatePhysicalCards(state) || hasDuplicateCardIds(state)) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  validateCardCounts(state)
}

// Each round's leader is forced by the rules: the dealer's opponent leads
// round 1 (the dealer is pe), and afterwards the previous round's winner --
// or its leader, on a tie -- leads. `firstRoundLeader` is derived from this
// history and decides all-three-tied hands, so a corrupted chain corrupts
// hand resolution.
function validateRoundLeaderChain(state: GameState): void {
  const rounds = state.completed_rounds
  if (rounds.length > 0 && rounds[0]!.leader !== otherPlayer(state.dealer)) {
    throw new TrucoEngineError('INVALID_INITIAL_STATE')
  }
  for (let i = 1; i < rounds.length; i += 1) {
    const current = rounds[i]!
    const previous = rounds[i - 1]!
    if (current.leader !== (previous.winner ?? previous.leader)) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }
}

// Card-count conservation: every completed round consumed exactly one card
// per player and a current-round play consumes one more, so each hand must
// hold exactly `3 - completed - playedNow` cards. (Folded hands keep their
// remaining cards, so this holds for finished states too.)
function validateCardCounts(state: GameState): void {
  for (const player of [0, 1] as const) {
    const playedNow = state.current_round.plays.filter((play) => play.player === player).length
    const expected = 3 - state.completed_rounds.length - playedNow
    if (expected < 0 || state.hands[player].length !== expected) {
      throw new TrucoEngineError('INVALID_INITIAL_STATE')
    }
  }
}

// Card ids must be unique: `applyPlay` finds cards by id, so a duplicated id
// makes action application ambiguous.
function hasDuplicateCardIds(state: GameState): boolean {
  const seen = new Set<string>()
  for (const card of allPhysicalCardsInState(state)) {
    if (seen.has(card.id)) {
      return true
    }
    seen.add(card.id)
  }
  return false
}

function comparePlays(turnup: Turnup, first: PlayedCard, second: PlayedCard): Player | null {
  if (first.visibility === 'down' && second.visibility === 'down') {
    return null
  }
  if (first.visibility === 'down' && second.visibility === 'up') {
    return second.player
  }
  if (first.visibility === 'up' && second.visibility === 'down') {
    return first.player
  }

  const comparison = compareFaceUpCards(turnup, first.card, second.card)
  if (comparison > 0) return first.player
  if (comparison < 0) return second.player
  return null
}

function decideHandWinner(rounds: CompletedRound[], leaderOfFirstRound: Player): Player | null {
  const winsZero = rounds.filter((round) => round.winner === 0).length
  const winsOne = rounds.filter((round) => round.winner === 1).length

  if (winsZero >= 2) return 0
  if (winsOne >= 2) return 1

  if (rounds.length >= 2) {
    if (winsZero === 1 && winsOne === 0 && rounds.some((round) => round.winner == null)) {
      return 0
    }
    if (winsOne === 1 && winsZero === 0 && rounds.some((round) => round.winner == null)) {
      return 1
    }
  }

  if (rounds.length === 3) {
    if (winsZero === 1 && winsOne === 1) {
      return rounds.find((round) => round.winner != null)?.winner ?? null
    }
    return leaderOfFirstRound
  }

  return null
}

function expectedRoundLeader(dealer: Player, completedRounds: CompletedRound[]): Player {
  const lastRound = completedRounds[completedRounds.length - 1]
  return lastRound == null ? otherPlayer(dealer) : (lastRound.winner ?? lastRound.leader)
}

function firstRoundLeader(dealer: Player, completedRounds: CompletedRound[]): Player {
  return completedRounds[0]?.leader ?? otherPlayer(dealer)
}

function nextRaiseAfter(value: number): number | null {
  const index = RAISE_LADDER.findIndex((step) => step === value)
  return index < 0 ? null : RAISE_LADDER[index + 1] ?? null
}

function otherPlayer(player: Player): Player {
  return player === 0 ? 1 : 0
}

function scoreTriggersElevenHand(score: Score): boolean {
  return score[0] === 11 || score[1] === 11
}

function bothPlayersOnEleven(score: Score): boolean {
  return score[0] === 11 && score[1] === 11
}

function startingHandValue(score: Score): number {
  return bothPlayersOnEleven(score) ? ELEVEN_AUTO_HAND_VALUE : INITIAL_HAND_VALUE
}

function elevenHandDecisionPlayer(score: Score): Player | null {
  if (score[0] === 11 && score[1] !== 11) return 0
  if (score[0] !== 11 && score[1] === 11) return 1
  return null
}

function elevenFoldAward(): number {
  return INITIAL_HAND_VALUE
}

function scoreWinner(score: Score): Player | null {
  if (score[0] >= MATCH_TARGET) return 0
  if (score[1] >= MATCH_TARGET) return 1
  return null
}

function compareFaceUpCards(turnup: Turnup, first: Card, second: Card): number {
  const firstIsManilha = isManilha(first, turnup)
  const secondIsManilha = isManilha(second, turnup)

  if (firstIsManilha && !secondIsManilha) return 1
  if (!firstIsManilha && secondIsManilha) return -1
  if (firstIsManilha && secondIsManilha) {
    return Math.sign(manilhaStrength(first.suit) - manilhaStrength(second.suit))
  }
  return Math.sign(rankIndex(first.rank) - rankIndex(second.rank))
}

function rankIndex(rank: Rank): number {
  return RANKS.indexOf(rank)
}

function nextRankForManilha(rank: Rank): Rank {
  const index = rankIndex(rank)
  return RANKS[(index + 1) % RANKS.length]!
}

function manilhaStrength(suit: Suit): number {
  return SUITS.indexOf(suit)
}

function isManilha(card: Card, turnup: Turnup): boolean {
  return card.rank === nextRankForManilha(turnup.rank)
}

function hasDuplicatePhysicalCards(state: GameState): boolean {
  const seen = new Set<string>([physicalCardKey(state.turnup)])
  for (const card of allPhysicalCardsInState(state)) {
    const key = physicalCardKey(card)
    if (seen.has(key)) {
      return true
    }
    seen.add(key)
  }
  return false
}

function allPhysicalCardsInState(state: GameState): Card[] {
  return [
    ...state.hands[0],
    ...state.hands[1],
    ...state.completed_rounds.flatMap((round) => (round.plays ?? []).map((play) => play.card)),
    ...state.current_round.plays.map((play) => play.card),
  ]
}

function physicalCardKey(card: Pick<Card, 'rank' | 'suit'>): string {
  return `${card.rank}:${card.suit}`
}

function validCompletedRoundHistory(round: CompletedRound, state: GameState): boolean {
  const plays = round.plays ?? []
  if (plays.length === 0) {
    return true
  }
  if (plays.length !== 2) {
    return false
  }
  const first = plays[0]!
  const second = plays[1]!
  return first.player === round.leader
    && isPlayer(first.player)
    && isPlayer(second.player)
    && first.player !== second.player
    && comparePlays(state.turnup, first, second) === (round.winner ?? null)
}

function raiseValueReachableFrom(current: number, target: number): boolean {
  let step: number | null = current
  while (step != null) {
    if (step === target) {
      return true
    }
    step = nextRaiseAfter(step)
  }
  return false
}

function isPlayer(player: unknown): player is Player {
  return player === 0 || player === 1
}

function sameOptionalPlayer(left: Player | null | undefined, right: Player | null | undefined): boolean {
  return (left ?? null) === (right ?? null)
}

function playerHand(state: GameState, player: Player): Card[] {
  return player === 0 ? state.hands[0] : state.hands[1]
}

function mutablePlayerHand(state: GameState, player: Player): Card[] {
  return player === 0 ? state.hands[0] : state.hands[1]
}

function filterStrategicActions(actions: Action[]): Action[] {
  return actions.filter((action) => action.type !== 'concede_hand')
}

function actionEquals(left: Action, right: Action): boolean {
  return jsonEquals(left, right)
}

function expectedSubsetMismatch(actual: unknown, expected: unknown): string | null {
  if (isPlainObject(actual) && isPlainObject(expected)) {
    for (const [key, expectedValue] of Object.entries(expected)) {
      if (!(key in actual)) {
        return `missing key \`${key}\` in actual state`
      }
      const childMessage = expectedSubsetMismatch(actual[key], expectedValue)
      if (childMessage != null) {
        return `${key}: ${childMessage}`
      }
    }
    return null
  }

  if (Array.isArray(actual) && Array.isArray(expected)) {
    if (actual.length !== expected.length) {
      return `array length mismatch: expected ${expected.length}, got ${actual.length}`
    }
    for (const [index, expectedValue] of expected.entries()) {
      const childMessage = expectedSubsetMismatch(actual[index], expectedValue)
      if (childMessage != null) {
        return `[${index}]: ${childMessage}`
      }
    }
    return null
  }

  if (jsonEquals(actual, expected)) {
    return null
  }

  return `expected ${JSON.stringify(expected)}, got ${JSON.stringify(actual)}`
}

function isPlainObject(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value != null && !Array.isArray(value)
}

function engineErrorCode(error: unknown): EngineErrorCode | null {
  return error instanceof TrucoEngineError ? error.code : null
}

function jsonEquals(left: unknown, right: unknown): boolean {
  return JSON.stringify(left) === JSON.stringify(right)
}

function prettyJson(value: unknown): string {
  return JSON.stringify(value, null, 2)
}

function clone<T>(value: T): T {
  return JSON.parse(JSON.stringify(value)) as T
}

const RANKS: Rank[] = ['4', '5', '6', '7', 'Q', 'J', 'K', 'A', '2', '3']
const SUITS: Suit[] = ['DIAMONDS', 'SPADES', 'HEARTS', 'CLUBS']
