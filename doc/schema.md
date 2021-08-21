认证：请求的 `token` header为用户的token

```
type AcFunUserInfo {
        userId: Int!
        nickname: String!
}
type Comment {
        liveId: String!
        sendTime: Int!
        userInfo: UserInfo
        content: String!
}
type Follow {
        liveId: String!
        sendTime: Int!
        userInfo: UserInfo
}
type Gift {
        liveId: String!
        sendTime: Int!
        userInfo: UserInfo
        giftId: Int!
        count: Int!
        combo: Int!
        value: Int!
        comboId: String!
        slotDisplayDuration: Int!
        expireDuration: Int!
        drawGiftInfo: String
}
type GiftInfo {
        id: Int
        saveTime: Int
        giftId: Int!
        giftName: String!
        arLiveName: String!
        payWalletType: Int!
        giftPrice: Int!
        webpPic: String
        pngPic: String
        smallPngPic: String
        allowBatchSendSizeList: String
        canCombo: Boolean!
        canDraw: Boolean!
        magicFaceId: Int!
        vupArId: Int!
        description: String!
        redpackPrice: Int!
        cornerMarkerText: String!
}
type JoinClub {
        liveId: String!
        joinTime: Int!
        fansInfo: AcFunUserInfo
        uperInfo: AcFunUserInfo
}
type Live {
        liveId: String!
        liverUid: Int!
        nickname: String!
        streamName: String!
        startTime: Int!
        title: String
        liveType: LiveType
        portrait: Boolean
        panoramic: Boolean
        disableDanmakuShow: Boolean
        duration: Int
        likeCount: String
        watchCount: String
}
type LiveInfo {
        liveId: String!
        liverUid: Int!
        liverInfo: LiverInfo
        streamName: String!
        startTime: Int!
        title: [Title!]
        cover: String
        liveType: LiveType
        hasFansClub: Boolean!
        portrait: Boolean!
        panoramic: Boolean!
        disableDanmakuShow: Boolean!
        paidShowUserBuyStatus: Boolean!
        summary: Summary
}
type LiveType {
        id: Int!
        name: String!
        categoryId: Int!
        categoryName: String!
}
type LiverInfo {
        liveId: String!
        saveTime: Int!
        liverUid: Int!
        nickname: String!
        avatar: String!
        avatarFrame: String!
        followingCount: Int!
        contributeCount: Int!
        liveBeginFansCount: Int!
        liveEndFansCount: Int
        signature: String
        verifiedText: String
        isJoinUpCollege: Boolean
        medalName: String
        liveBeginMedalCount: Int
        liveEndMedalCount: Int
}
type MedalInfo {
        uperUid: Int!
        name: String!
        level: Int!
}
type QueryRoot {
        addLiver(liverUid: Int!): TokenInfo!
        deleteLiver(liverUid: Int!): TokenInfo!
        liverUid: Int!
        live(liveId: [String!], liverUid: [Int!], start: Int, end: Int): [Live!]!
        giftInfo(giftId: [Int!], allHistory: Boolean): [GiftInfo!]!
        liveInfo(liveId: [String!], start: Int, end: Int, liverUid: Int): [LiveInfo!]!
        title(liveId: [String!], start: Int, end: Int, liverUid: Int): [Title!]!
        liverInfo(liveId: [String!], start: Int, end: Int, liverUid: Int): [LiverInfo!]!
        summary(liveId: [String!], start: Int, end: Int, liverUid: Int): [Summary!]!
        comment(liveId: [String!], userId: [Int!], start: Int, end: Int, liverUid: Int): [Comment!]!
        follow(liveId: [String!], start: Int, end: Int, liverUid: Int): [Follow!]!
        gift(liveId: [String!], userId: [Int!], giftId: [Int!], start: Int, end: Int, liverUid: Int): [Gift!]!
        joinClub(liveId: [String!], start: Int, end: Int, liverUid: Int): [JoinClub!]!
        watchingCount(liveId: [String!], start: Int, end: Int, liverUid: Int): [WatchingCount!]!
}
type Summary {
        liveId: String!
        saveTime: Int!
        duration: Int!
        likeCount: String!
        watchTotalCount: String!
        watchOnlineMaxCount: Int
        bananaCount: String
}
type Title {
        liveId: String!
        saveTime: Int!
        title: String
}
type TokenInfo {
        exist: Boolean!
        token: String
}
type UserInfo {
        userId: Int!
        nickname: String!
        avatar: String
        medal: MedalInfo
        manager: Boolean
}
type WatchingCount {
        liveId: String!
        saveTime: Int!
        watchingCount: Int
}
schema {
        query: QueryRoot
}
```
